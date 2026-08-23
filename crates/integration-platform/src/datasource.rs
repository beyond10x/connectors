use super::*;
use serde_json::json;

const LIST_OPERATION: &str = "workspaces-list";
const GET_OPERATION: &str = "workspaces-get";
const PROJECTION_PROTOCOL: &str = "b10x.value-projection.v1";

impl PlatformBackend {
    pub(super) fn workspace_datasource_admitted(&self) -> bool {
        self.config.module_configured("workspaces")
            && self.module_admitted("workspaces")
            && self.configured(LIST_OPERATION)
            && self.configured(GET_OPERATION)
    }

    pub(super) async fn handle_workspace_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        if !self.context_admitted(context) {
            return Err(datasource_error(
                DatasourceErrorCode::StaleAuthority,
                "workspace datasource authority is stale",
                false,
            ));
        }
        match request {
            DatasourceRequest::Search(DatasourceSearchRequest { query, limit }) => {
                let query = query.to_ascii_lowercase();
                let definitions = (self.workspace_datasource_admitted()
                    && (query.is_empty()
                        || ["b10x", "workspace", "repository", "development"]
                            .iter()
                            .any(|term| query.contains(term))))
                .then(workspace_datasource_summary)
                .into_iter()
                .take(usize::from(limit))
                .collect();
                Ok(DatasourceResult::Search { definitions })
            }
            DatasourceRequest::Describe(DatasourceDescribeRequest { datasource_ref })
                if datasource_ref == WORKSPACES_DATASOURCE
                    && self.workspace_datasource_admitted() =>
            {
                Ok(DatasourceResult::Describe(
                    self.workspace_datasource_description(context),
                ))
            }
            DatasourceRequest::Bindings(BindingSearchRequest {
                datasource_ref,
                query,
                limit,
            }) if datasource_ref == WORKSPACES_DATASOURCE
                && self.workspace_datasource_admitted() =>
            {
                let binding = self.workspace_datasource_binding();
                let query = query.to_ascii_lowercase();
                // The query narrows an already-admitted binding; it is a hint, never a
                // permission boundary. A topical query ("list") used to hide the only binding
                // that exists and read as "this datasource is not bound".
                let _ = &query;
                let bindings = Some(binding).into_iter().take(usize::from(limit)).collect();
                Ok(DatasourceResult::Bindings { bindings })
            }
            DatasourceRequest::Read(request) if self.workspace_datasource_admitted() => {
                self.read_workspace_datasource(context, request).await
            }
            // The three non-search verbs are gated by the same admission, so they cannot disagree
            // about whether this datasource is available. Say which of the two things is true
            // rather than "was not found" for both.
            DatasourceRequest::Describe(DatasourceDescribeRequest { datasource_ref })
            | DatasourceRequest::Bindings(BindingSearchRequest { datasource_ref, .. })
                if datasource_ref == WORKSPACES_DATASOURCE =>
            {
                Err(workspace_module_unavailable())
            }
            DatasourceRequest::Read(request) if request.datasource_ref == WORKSPACES_DATASOURCE => {
                Err(workspace_module_unavailable())
            }
            _ => Err(datasource_error(
                DatasourceErrorCode::NotFound,
                "workspace datasource was not found",
                false,
            )),
        }
    }

    async fn read_workspace_datasource(
        &self,
        context: &PrincipalContext,
        request: ReadRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        // Three unrelated facts used to share one `StaleAuthority`, so the one message a person
        // saw could mean the wrong datasource, a lease that has moved on, or a binding ref the
        // caller made up. A reviewing agent read it as the last kind of problem it could be and
        // asked an operator to re-authorize a lease that was current.
        if request.datasource_ref != WORKSPACES_DATASOURCE {
            return Err(datasource_error(
                DatasourceErrorCode::NotFound,
                "workspace datasource was not found",
                false,
            ));
        }
        if request.binding_ref != self.workspace_datasource_binding_ref() {
            return Err(DatasourceError::new(
                DatasourceErrorCode::InvalidInput,
                format!(
                    "`{}` is not a binding of `{WORKSPACES_DATASOURCE}`; list its bindings and read through one of those",
                    request.binding_ref
                ),
                false,
            ));
        }
        if request.description_ref != self.workspace_datasource_description_ref(context) {
            return Err(datasource_error(
                DatasourceErrorCode::StaleAuthority,
                "the workspace datasource description lease has moved on; describe it again and retry the read",
                true,
            ));
        }
        let binding_ref = request.binding_ref.clone();
        let (records, next_cursor, audit_ref) = match request.read {
            DatasourceRead::List { limit, cursor } => {
                let offset = self.workspace_cursor_offset(context, cursor.as_deref())?;
                let (output, audit_ref) = self
                    .invoke_datasource_operation(context, LIST_OPERATION, json!({}))
                    .await?;
                let mut items = output
                    .pointer("/result/items")
                    .and_then(Value::as_array)
                    .cloned()
                    .ok_or_else(datasource_protocol)?;
                items.sort_by(|left, right| {
                    left.get("id")
                        .and_then(Value::as_str)
                        .cmp(&right.get("id").and_then(Value::as_str))
                });
                let start = offset.min(items.len());
                let end = start.saturating_add(usize::from(limit)).min(items.len());
                let records = items[start..end]
                    .iter()
                    .map(|workspace| workspace_record(workspace, RecordView::Compact))
                    .collect::<Result<Vec<_>, _>>()?;
                let next = (end < items.len()).then(|| self.workspace_cursor(context, end));
                (records, next, audit_ref)
            }
            DatasourceRead::Get { key } => {
                let workspace = workspace_key(key)?;
                let (output, audit_ref) = self
                    .invoke_datasource_operation(
                        context,
                        GET_OPERATION,
                        json!({"workspace": workspace}),
                    )
                    .await?;
                let value = output.get("result").ok_or_else(datasource_protocol)?;
                (
                    vec![workspace_record(value, RecordView::Detail)?],
                    None,
                    audit_ref,
                )
            }
        };
        Ok(DatasourceResult::Read(DatasourcePage {
            datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
            records,
            next_cursor,
            completeness: Completeness::Complete,
            observed_at_unix_ms: now_unix_ms(),
            provenance: DatasourceProvenance {
                binding_ref,
                projection_sha256: workspace_projection_sha256(),
                connector_audit_ref: audit_ref,
            },
        }))
    }

    async fn invoke_datasource_operation(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
        input: Value,
    ) -> Result<(Value, String), DatasourceError> {
        let canonical = self
            .operation(operation_ref)
            .ok_or_else(|| datasource_not_granted("workspace datasource operation is unavailable"))?
            .canonical;
        let result = self
            .invoke(
                context,
                InvokeRequest {
                    operation_ref: operation_ref.to_owned(),
                    connection_ref: self.config.connection.connection_ref.clone(),
                    description_ref: self.description_ref(context, canonical),
                    input,
                    approval_evidence_ref: None,
                },
            )
            .await
            .map_err(datasource_from_operation)?;
        match result {
            OperationResult::Invoke(result) => Ok((result.output, result.connector_audit_ref)),
            _ => Err(datasource_protocol()),
        }
    }

    fn workspace_datasource_description(
        &self,
        context: &PrincipalContext,
    ) -> DatasourceDescription {
        DatasourceDescription {
            summary: workspace_datasource_summary(),
            description: "Live logical workspace catalog for the authenticated owner scope. It projects lifecycle and credential-free repository identity only; tenant, principal, Connector references, checkout paths, files, and file contents are excluded."
                .to_owned(),
            key_schema: workspace_key_schema(),
            compact_schema: workspace_compact_schema(),
            detail_schema: workspace_detail_schema(),
            projection_protocol: PROJECTION_PROTOCOL.to_owned(),
            projection_sha256: workspace_projection_sha256(),
            description_ref: self.workspace_datasource_description_ref(context),
        }
    }

    /// The description lease for the workspace datasource.
    ///
    /// Seeded from the stable authority — the same seed the operation lease in this backend
    /// already uses — and never from the authority snapshot id or sha. The hosted receiver fills
    /// those from the access token, so they rotate every few minutes; every datasource request
    /// travels on the cached `connectors.catalog.read` token, and a describe and a read that
    /// straddle its refresh arrive on different tokens. Seeding the lease from them made the
    /// deployed read fail with `StaleAuthority` at one minute and succeed at the next.
    fn workspace_datasource_description_ref(&self, context: &PrincipalContext) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/workspace-datasource-description/v2\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(workspace_projection_sha256().as_bytes());
        digest.update(b"\0");
        digest.update(self.deployment_sha256.as_bytes());
        let digest = digest.finalize();
        format!(
            "description:b10x:workspaces:{}",
            hex::encode(&digest[..16])
        )
    }

    fn workspace_datasource_binding(&self) -> DatasourceBinding {
        let digest = Sha256::digest(format!(
            "{WORKSPACES_DATASOURCE}\0{}\0{}\0binding-v1",
            self.config.connection.connection_ref, self.deployment_sha256,
        ));
        let mut generation = [0_u8; 8];
        generation.copy_from_slice(&digest[..8]);
        DatasourceBinding {
            datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
            binding_ref: self.workspace_datasource_binding_ref(),
            connection_ref: self.config.connection.connection_ref.clone(),
            label: "platform logical workspaces".to_owned(),
            purpose: None,
            generation: u64::from_be_bytes(generation).max(1),
        }
    }

    fn workspace_datasource_binding_ref(&self) -> String {
        let digest = Sha256::digest(format!(
            "{WORKSPACES_DATASOURCE}\0{}\0{}\0binding-ref-v1",
            self.config.connection.connection_ref, self.deployment_sha256,
        ));
        format!(
            "binding:b10x:workspaces:{}",
            hex::encode(&digest[..16])
        )
    }

    fn workspace_cursor(&self, context: &PrincipalContext, offset: usize) -> String {
        let digest = self.workspace_cursor_digest(context, offset);
        format!("cursor:b10x:workspaces:{offset}:{digest}")
    }

    fn workspace_cursor_offset(
        &self,
        context: &PrincipalContext,
        cursor: Option<&str>,
    ) -> Result<usize, DatasourceError> {
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        let fields = cursor.split(':').collect::<Vec<_>>();
        let ["cursor", "b10x", "workspaces", offset, digest] = fields.as_slice() else {
            return Err(datasource_cursor_expired());
        };
        let offset = offset
            .parse::<usize>()
            .map_err(|_| datasource_cursor_expired())?;
        if *digest != self.workspace_cursor_digest(context, offset) {
            return Err(datasource_cursor_expired());
        }
        Ok(offset)
    }

    /// The page cursor is bound to the same stable authority as the lease, for the same reason:
    /// page two of a listing arrives on whatever access token is current then, and an expired
    /// cursor for a rotated token is a refusal a caller cannot act on.
    fn workspace_cursor_digest(&self, context: &PrincipalContext, offset: usize) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/workspace-datasource-cursor/v2\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(workspace_projection_sha256().as_bytes());
        digest.update(b"\0");
        digest.update(self.deployment_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(offset.to_be_bytes());
        hex::encode(&digest.finalize()[..16])
    }
}

fn workspace_datasource_summary() -> DatasourceSummary {
    DatasourceSummary {
        datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
        title: "platform workspaces".to_owned(),
        access_mode: AccessMode::Live,
        verbs: vec![ReadVerb::List, ReadVerb::Get],
    }
}

fn workspace_key_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["id"],
        "properties": {"id": {"type": "string", "minLength": 1, "maxLength": 128}}
    })
}

fn workspace_source_schema() -> Value {
    json!({
        "type": ["object", "null"],
        "additionalProperties": false,
        "required": ["forge", "canonical_url", "repository", "default_branch"],
        "properties": {
            "forge": {"enum": ["git-hub", "git-lab"]},
            "canonical_url": {"type": "string"},
            "repository": {"type": "string"},
            "default_branch": {"type": "string"}
        }
    })
}

fn workspace_compact_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["id", "name", "retention", "source", "state", "updated_at_ms", "expires_at_ms"],
        "properties": {
            "id": {"type": "string"},
            "name": {"type": "string"},
            "retention": {"enum": ["managed", "ephemeral"]},
            "source": workspace_source_schema(),
            "state": {"enum": ["active", "destroying"]},
            "updated_at_ms": {"type": "integer", "minimum": 0},
            "expires_at_ms": {"type": ["integer", "null"], "minimum": 0}
        }
    })
}

fn workspace_detail_schema() -> Value {
    let mut schema = workspace_compact_schema();
    let object = schema.as_object_mut().expect("static workspace schema");
    object
        .get_mut("required")
        .and_then(Value::as_array_mut)
        .expect("static required schema")
        .push(json!("created_at_ms"));
    object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .expect("static properties schema")
        .insert(
            "created_at_ms".to_owned(),
            json!({"type": "integer", "minimum": 0}),
        );
    schema
}

fn workspace_projection_sha256() -> String {
    let declaration = json!({
        "protocol": PROJECTION_PROTOCOL,
        "datasource_ref": WORKSPACES_DATASOURCE,
        "version": 1,
        "key_schema": workspace_key_schema(),
        "compact_schema": workspace_compact_schema(),
        "detail_schema": workspace_detail_schema(),
        "excluded": ["tenant", "owner", "source.connection", "checkouts", "paths", "files", "file_contents"]
    });
    hex::encode(Sha256::digest(
        serde_json::to_vec(&declaration).expect("static workspace projection declaration"),
    ))
}

fn workspace_key(key: Value) -> Result<String, DatasourceError> {
    let fields = key.as_object().ok_or_else(datasource_invalid)?;
    if fields.len() != 1 {
        return Err(datasource_invalid());
    }
    fields
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| {
            !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
        .map(ToOwned::to_owned)
        .ok_or_else(datasource_invalid)
}

fn workspace_record(value: &Value, view: RecordView) -> Result<DatasourceRecord, DatasourceError> {
    let field = |name| value.get(name).cloned().ok_or_else(datasource_protocol);
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(datasource_protocol)?
        .to_owned();
    let source = match value.get("source") {
        Some(Value::Null) | None => Value::Null,
        Some(source) => json!({
            "forge": source.get("forge").cloned().ok_or_else(datasource_protocol)?,
            "canonical_url": source.get("canonical_url").cloned().ok_or_else(datasource_protocol)?,
            "repository": source.get("repository").cloned().ok_or_else(datasource_protocol)?,
            "default_branch": source.get("default_branch").cloned().ok_or_else(datasource_protocol)?,
        }),
    };
    let mut projected = serde_json::Map::from_iter([
        ("id".to_owned(), json!(id)),
        ("name".to_owned(), field("name")?),
        ("retention".to_owned(), field("retention")?),
        ("source".to_owned(), source),
        ("state".to_owned(), field("state")?),
        ("updated_at_ms".to_owned(), field("updated_at_ms")?),
        (
            "expires_at_ms".to_owned(),
            value.get("expires_at_ms").cloned().unwrap_or(Value::Null),
        ),
    ]);
    if view == RecordView::Detail {
        projected.insert("created_at_ms".to_owned(), field("created_at_ms")?);
    }
    Ok(DatasourceRecord {
        key: json!({"id": id}),
        view,
        value: Value::Object(projected),
    })
}

fn datasource_from_operation(error: OperationError) -> DatasourceError {
    let code = match error.code {
        OperationErrorCode::NotFound => DatasourceErrorCode::NotFound,
        OperationErrorCode::NotGranted
        | OperationErrorCode::ApprovalDenied
        | OperationErrorCode::ApprovalRequired => DatasourceErrorCode::NotGranted,
        OperationErrorCode::InvalidInput => DatasourceErrorCode::InvalidInput,
        OperationErrorCode::StaleAuthority => DatasourceErrorCode::StaleAuthority,
        OperationErrorCode::ResultTooLarge => DatasourceErrorCode::ResultTooLarge,
        OperationErrorCode::Protocol => DatasourceErrorCode::Protocol,
        OperationErrorCode::Unavailable | OperationErrorCode::OutcomeUnknown => {
            DatasourceErrorCode::Unavailable
        }
    };
    datasource_error(code, "workspace owner read failed", error.retriable)
}

fn datasource_protocol() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::Protocol,
        "workspace owner returned an incompatible projection",
        false,
    )
}

fn datasource_invalid() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::InvalidInput,
        "workspace datasource key is invalid",
        false,
    )
}

fn datasource_not_granted(message: &'static str) -> DatasourceError {
    datasource_error(DatasourceErrorCode::NotGranted, message, false)
}

/// The workspaces module is not admitted or not configured for this principal.
///
/// Retriable: a module that is starting, or a tenant module list that has not landed, clears
/// without any change a caller can make. Terminal-sounding wording here sent a reviewing agent to
/// an operator for a datasource that simply was not up yet.
fn workspace_module_unavailable() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::Unavailable,
        "the workspaces module is not admitted or not answering for this principal, so the workspace datasource cannot be served right now; retry, and if it persists ask an operator whether the workspaces module is enabled for this tenant",
        true,
    )
}

fn datasource_cursor_expired() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::CursorExpired,
        "workspace datasource cursor is stale or belongs to different authority",
        false,
    )
}

fn datasource_error(
    code: DatasourceErrorCode,
    message: &'static str,
    retriable: bool,
) -> DatasourceError {
    DatasourceError::new(code, message, retriable)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
