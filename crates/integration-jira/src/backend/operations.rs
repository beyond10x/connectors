//! Delegated-user Jira operation discovery, project admission, and one request-composition path.

use std::collections::BTreeMap;

use protocol::operation::{
    ApprovalPosture, ConnectionSummary as OperationConnectionSummary, EffectClass,
    InvocationResult, InvokeRequest, OperationDescription, OperationError, OperationErrorCode,
    OperationResult, OperationSummary,
};
use serde_json::{json, Value};
use service::PrincipalContext;
use sha2::{Digest as _, Sha256};

use super::auth::decode_value_response;
use super::*;

impl JiraInner {
    fn operation_connections(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Vec<OperationConnectionSummary> {
        self.owned_user_connections(context)
            .into_iter()
            .filter(|connection| supports_operation(connection, operation_ref))
            .map(|connection| OperationConnectionSummary {
                connection_ref: connection.connection_ref,
                label: connection.label,
                provider: INTEGRATION_REF.to_owned(),
                audiences: vec!["delegated-user".to_owned()],
            })
            .collect()
    }

    pub(super) fn search_operations(
        &self,
        context: &PrincipalContext,
        query: &str,
    ) -> Vec<OperationSummary> {
        let query = query.to_ascii_lowercase();
        JIRA_OPERATIONS
            .iter()
            .filter_map(|operation_ref| {
                let connections = self.operation_connections(context, operation_ref);
                if connections.is_empty() {
                    return None;
                }
                let operation = connector_resolve::document::operation(operation_ref)?;
                (query.is_empty()
                    || operation_ref.contains(&query)
                    || operation
                        .contract_description()
                        .to_ascii_lowercase()
                        .contains(&query))
                .then(|| OperationSummary {
                    operation_ref: (*operation_ref).to_owned(),
                    title: operation_ref.replace('-', " "),
                    effect: operation_effect(operation_ref),
                    approval: operation_approval(operation_ref),
                    connections,
                })
            })
            .collect()
    }

    fn operation_description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/jira-operation-description/v1\0");
        digest.update(serde_json::to_vec(context).expect("principal context serializes"));
        digest.update(b"\0");
        digest.update(operation_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.policy.user_grant_ref.as_bytes());
        for project in &self.policy.allowed_project_keys {
            digest.update(b"\0");
            digest.update(project.as_bytes());
        }
        for connection in self.operation_connections(context, operation_ref) {
            digest.update(b"\0");
            digest.update(connection.connection_ref.as_bytes());
        }
        format!("description-sha256-{:x}", digest.finalize())
    }

    pub(super) fn describe_operation(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<OperationResult, OperationError> {
        if !is_jira_operation(operation_ref) {
            return Err(operation_not_found());
        }
        let operation = connector_resolve::document::operation(operation_ref)
            .ok_or_else(operation_not_found)?;
        let connections = self.operation_connections(context, operation_ref);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: operation_ref.replace('-', " "),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: operation_output_schema(operation_ref),
            effect: operation_effect(operation_ref),
            approval: operation_approval(operation_ref),
            connections,
            description_ref: self.operation_description_ref(context, operation_ref),
        }))
    }

    pub(super) async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if !is_jira_operation(&request.operation_ref) {
            return Err(operation_not_found());
        }
        let connection = self
            .owned_user_connections(context)
            .into_iter()
            .find(|connection| {
                connection.connection_ref == request.connection_ref
                    && supports_operation(connection, &request.operation_ref)
            })
            .ok_or_else(operation_not_granted)?;
        if request.description_ref
            != self.operation_description_ref(context, &request.operation_ref)
        {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "Jira operation description lease is stale",
                false,
            ));
        }
        if operation_approval(&request.operation_ref) == ApprovalPosture::Required
            && request.approval_evidence_ref.is_none()
        {
            return Err(OperationError::new(
                OperationErrorCode::ApprovalRequired,
                "this Jira write requires correlated approval evidence",
                false,
            ));
        }
        self.admit_operation_projects(&request.operation_ref, &request.input)?;
        let operation = connector_resolve::document::operation(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        let validator = jsonschema::validator_for(operation.input_schema())
            .map_err(|_| operation_unavailable())?;
        if !validator.is_valid(&request.input) {
            return Err(operation_invalid());
        }
        let token = self
            .user_access_token(&connection)
            .await
            .map_err(|_| operation_not_granted())?;
        let assembled = connector_resolve::auth::Assembled::new(
            "jira.user_oauth",
            token.expose_secret().to_owned(),
            catalog::Placement::Header {
                name: "Authorization",
                prefix: "Bearer ",
            },
        );
        drop(token);
        let base = self.gateway_origin.as_str().trim_end_matches('/');
        let plan = connector_resolve::resolve(
            operation,
            base,
            &request.input,
            &BTreeMap::new(),
            &[assembled],
        )
        .map_err(|_| operation_invalid())?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| operation_unavailable())?;
        if !self.admitted_gateway_target(&target) {
            return Err(operation_not_granted());
        }
        let method = reqwest::Method::from_bytes(plan.request.method.as_bytes())
            .map_err(|_| operation_unavailable())?;
        let mut outbound = self
            .http
            .request(method, target)
            .header("Accept", "application/json");
        for (name, value) in plan.request.headers {
            outbound = outbound.header(name, value);
        }
        if let Some(body) = plan.request.body {
            outbound = outbound
                .header("Content-Type", "application/json")
                .body(body);
        }
        let audit_ref = format!(
            "audit:jira:{}",
            random_uuid().map_err(|_| operation_unavailable())?
        );
        self.audit(
            &audit_ref,
            &request.operation_ref,
            &request.connection_ref,
            context,
            "attempted",
        )
        .map_err(|_| operation_unavailable())?;
        let response = outbound.send().await;
        let output = match response {
            Ok(response) if response.status().is_success() => {
                decode_value_response(response, protocol::operation::MAX_RESULT_BYTES)
                    .await
                    .map_err(|_| {
                        if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                            operation_unavailable()
                        } else {
                            operation_outcome_unknown(&request.operation_ref)
                        }
                    })
                    .and_then(|payload| {
                        project_operation_output(
                            &request.operation_ref,
                            &payload,
                            &self.site_origin,
                            &request.input,
                        )
                        .and_then(|projected| {
                            let schema = operation_output_schema(&request.operation_ref);
                            let validator = jsonschema::validator_for(&schema)
                                .map_err(|_| operation_unavailable())?;
                            validator
                                .is_valid(&projected)
                                .then_some(projected)
                                .ok_or_else(operation_protocol)
                        })
                        .map_err(|_| {
                            if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                                operation_protocol()
                            } else {
                                operation_outcome_unknown(&request.operation_ref)
                            }
                        })
                    })
            }
            Ok(response) if response.status().is_client_error() => Err(
                if matches!(
                    response.status(),
                    reqwest::StatusCode::UNAUTHORIZED
                        | reqwest::StatusCode::FORBIDDEN
                        | reqwest::StatusCode::NOT_FOUND
                ) {
                    operation_not_granted()
                } else {
                    operation_invalid()
                },
            ),
            Ok(_) | Err(_) => Err(
                if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                    operation_unavailable()
                } else {
                    operation_outcome_unknown(&request.operation_ref)
                },
            ),
        };
        match output {
            Ok(output) => {
                self.audit(
                    &audit_ref,
                    &request.operation_ref,
                    &request.connection_ref,
                    context,
                    "completed",
                )
                .map_err(|_| operation_outcome_unknown(&request.operation_ref))?;
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output,
                    connector_audit_ref: audit_ref,
                    execution_ref: None,
                }))
            }
            Err(error) => {
                let _ = self.audit(
                    &audit_ref,
                    &request.operation_ref,
                    &request.connection_ref,
                    context,
                    "refused_or_indeterminate",
                );
                Err(error)
            }
        }
    }

    fn admit_operation_projects(
        &self,
        operation_ref: &str,
        input: &Value,
    ) -> Result<(), OperationError> {
        admit_operation_shape(operation_ref, input)?;
        let object = input.as_object().ok_or_else(operation_invalid)?;
        let keys: &[&str] = match operation_ref {
            "jira-issue-create" => &["project_key"],
            "jira-issue-link-add" => &["inward_issue_key", "outward_issue_key"],
            _ => &["issue_key"],
        };
        for field in keys {
            let value = object
                .get(*field)
                .and_then(Value::as_str)
                .ok_or_else(operation_invalid)?;
            let project = if *field == "project_key" {
                value
            } else {
                issue_project(value).ok_or_else(operation_invalid)?
            };
            if !self
                .policy
                .allowed_project_keys
                .iter()
                .any(|allowed| allowed == project)
            {
                return Err(operation_not_granted());
            }
        }
        Ok(())
    }

    pub(super) fn admitted_gateway_target(&self, target: &url::Url) -> bool {
        target.scheme() == self.gateway_origin.scheme()
            && target.host_str() == self.gateway_origin.host_str()
            && target.port_or_known_default() == self.gateway_origin.port_or_known_default()
            && target.username().is_empty()
            && target.password().is_none()
            && target.fragment().is_none()
            && target
                .path()
                .starts_with(&format!("/ex/jira/{}/rest/api/2/", self.policy.cloud_id))
    }
}

fn project_operation_output(
    operation_ref: &str,
    payload: &Value,
    site_origin: &url::Url,
    input: &Value,
) -> Result<Value, OperationError> {
    match operation_ref {
        "jira-issue-get" => super::datasource::project_issue(
            payload,
            protocol::datasource::RecordView::Detail,
            site_origin,
        )
        .and_then(|record| {
            (record.key == input["issue_key"])
                .then_some(record.value)
                .ok_or_else(|| {
                    protocol::datasource::DatasourceError::new(
                        protocol::datasource::DatasourceErrorCode::Protocol,
                        "Jira issue response did not match the requested key",
                        false,
                    )
                })
        })
        .map_err(|_| operation_protocol()),
        "jira-issue-comment-list" => project_comment_page(payload),
        "jira-issue-transitions-list" => project_transitions(payload),
        "jira-issue-create" => project_created_issue(payload, site_origin, input),
        "jira-issue-comment-add" | "jira-issue-comment-edit" => project_comment(payload),
        "jira-issue-transition" | "jira-issue-edit" | "jira-issue-link-add" => {
            Ok(json!({"completed": true}))
        }
        _ => Err(operation_protocol()),
    }
}

fn project_created_issue(
    payload: &Value,
    site_origin: &url::Url,
    input: &Value,
) -> Result<Value, OperationError> {
    let object = payload.as_object().ok_or_else(operation_protocol)?;
    let id = required_string(object.get("id"), 64)?;
    let key = required_string(object.get("key"), 64)?;
    if issue_project(&key) != input.get("project_key").and_then(Value::as_str) {
        return Err(operation_protocol());
    }
    Ok(json!({
        "id": id,
        "key": key,
        "browser_url": format!("{}/browse/{key}", site_origin.as_str().trim_end_matches('/')),
    }))
}

fn project_comment_page(payload: &Value) -> Result<Value, OperationError> {
    let object = payload.as_object().ok_or_else(operation_protocol)?;
    let comments = object
        .get("comments")
        .and_then(Value::as_array)
        .ok_or_else(operation_protocol)?
        .iter()
        .take(100)
        .map(project_comment)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "comments": comments,
        "start_at": bounded_integer(object.get("startAt"))?,
        "max_results": bounded_integer(object.get("maxResults"))?,
        "total": bounded_integer(object.get("total"))?,
    }))
}

fn project_comment(payload: &Value) -> Result<Value, OperationError> {
    let object = payload.as_object().ok_or_else(operation_protocol)?;
    let author = object
        .get("author")
        .and_then(Value::as_object)
        .and_then(|author| author.get("displayName"))
        .and_then(Value::as_str)
        .map(|value| Value::String(bounded_string(value, 256)))
        .unwrap_or(Value::Null);
    Ok(json!({
        "id": required_string(object.get("id"), 64)?,
        "body": required_string(object.get("body"), 32_768)?,
        "author": author,
        "created": optional_string(object.get("created"), 64)?,
        "updated": optional_string(object.get("updated"), 64)?,
    }))
}

fn project_transitions(payload: &Value) -> Result<Value, OperationError> {
    let transitions = payload
        .as_object()
        .and_then(|object| object.get("transitions"))
        .and_then(Value::as_array)
        .ok_or_else(operation_protocol)?
        .iter()
        .take(100)
        .map(|transition| {
            let object = transition.as_object().ok_or_else(operation_protocol)?;
            let status = object
                .get("to")
                .and_then(Value::as_object)
                .and_then(|value| value.get("name"));
            Ok(json!({
                "id": required_string(object.get("id"), 32)?,
                "name": required_string(object.get("name"), 128)?,
                "to_status": required_string(status, 128)?,
            }))
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    Ok(json!({"transitions": transitions}))
}

fn required_string(value: Option<&Value>, maximum: usize) -> Result<String, OperationError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|value| bounded_string(value, maximum))
        .ok_or_else(operation_protocol)
}

fn optional_string(value: Option<&Value>, maximum: usize) -> Result<Value, OperationError> {
    match value {
        None | Some(Value::Null) => Ok(Value::Null),
        Some(value) => value
            .as_str()
            .map(|value| Value::String(bounded_string(value, maximum)))
            .ok_or_else(operation_protocol),
    }
}

fn bounded_integer(value: Option<&Value>) -> Result<u64, OperationError> {
    value
        .and_then(Value::as_u64)
        .filter(|value| *value <= u64::from(u32::MAX))
        .ok_or_else(operation_protocol)
}

fn operation_output_schema(operation_ref: &str) -> Value {
    let nullable_string = |maximum| json!({"type":["string","null"],"maxLength":maximum});
    let comment = || {
        json!({
            "type":"object","additionalProperties":false,
            "required":["id","body","author","created","updated"],
            "properties":{
                "id":{"type":"string","minLength":1,"maxLength":64},
                "body":{"type":"string","minLength":1,"maxLength":32768},
                "author":nullable_string(256),
                "created":nullable_string(64),
                "updated":nullable_string(64)
            }
        })
    };
    match operation_ref {
        "jira-issue-get" => super::datasource::datasource_declaration().2,
        "jira-issue-comment-list" => json!({
            "type":"object","additionalProperties":false,
            "required":["comments","start_at","max_results","total"],
            "properties":{
                "comments":{"type":"array","maxItems":100,"items":comment()},
                "start_at":{"type":"integer","minimum":0,"maximum":4294967295_u64},
                "max_results":{"type":"integer","minimum":0,"maximum":4294967295_u64},
                "total":{"type":"integer","minimum":0,"maximum":4294967295_u64}
            }
        }),
        "jira-issue-transitions-list" => json!({
            "type":"object","additionalProperties":false,"required":["transitions"],
            "properties":{"transitions":{"type":"array","maxItems":100,"items":{
                "type":"object","additionalProperties":false,
                "required":["id","name","to_status"],"properties":{
                    "id":{"type":"string","minLength":1,"maxLength":32},
                    "name":{"type":"string","minLength":1,"maxLength":128},
                    "to_status":{"type":"string","minLength":1,"maxLength":128}
                }
            }}}
        }),
        "jira-issue-create" => json!({
            "type":"object","additionalProperties":false,"required":["id","key","browser_url"],
            "properties":{
                "id":{"type":"string","minLength":1,"maxLength":64},
                "key":{"type":"string","minLength":3,"maxLength":64},
                "browser_url":{"type":"string","format":"uri","maxLength":2048}
            }
        }),
        "jira-issue-comment-add" | "jira-issue-comment-edit" => comment(),
        "jira-issue-transition" | "jira-issue-edit" | "jira-issue-link-add" => json!({
            "type":"object","additionalProperties":false,"required":["completed"],
            "properties":{"completed":{"const":true}}
        }),
        _ => json!({"type":"object","additionalProperties":false}),
    }
}

fn admit_operation_shape(operation_ref: &str, input: &Value) -> Result<(), OperationError> {
    let object = input.as_object().ok_or_else(operation_invalid)?;
    let fields: &[(&str, usize, FieldShape)] = match operation_ref {
        "jira-issue-get" | "jira-issue-comment-list" | "jira-issue-transitions-list" => {
            &[("issue_key", 64, FieldShape::IssueKey)]
        }
        "jira-issue-create" => &[
            ("project_key", 32, FieldShape::ProjectKey),
            ("summary", 255, FieldShape::OneLine),
            ("issue_type", 128, FieldShape::OneLine),
        ],
        "jira-issue-comment-add" => &[
            ("issue_key", 64, FieldShape::IssueKey),
            ("body", 32_768, FieldShape::Text),
        ],
        "jira-issue-transition" => &[
            ("issue_key", 64, FieldShape::IssueKey),
            ("transition_id", 32, FieldShape::NumericId),
        ],
        "jira-issue-edit" => &[
            ("issue_key", 64, FieldShape::IssueKey),
            ("summary", 255, FieldShape::OneLine),
        ],
        "jira-issue-comment-edit" => &[
            ("issue_key", 64, FieldShape::IssueKey),
            ("comment_id", 32, FieldShape::NumericId),
            ("body", 32_768, FieldShape::Text),
        ],
        "jira-issue-link-add" => &[
            ("link_type", 128, FieldShape::OneLine),
            ("inward_issue_key", 64, FieldShape::IssueKey),
            ("outward_issue_key", 64, FieldShape::IssueKey),
        ],
        _ => return Err(operation_invalid()),
    };
    if object.len() != fields.len()
        || fields.iter().any(|(field, maximum, shape)| {
            object
                .get(*field)
                .and_then(Value::as_str)
                .is_none_or(|value| !shape.admits(value, *maximum))
        })
    {
        return Err(operation_invalid());
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum FieldShape {
    IssueKey,
    NumericId,
    OneLine,
    ProjectKey,
    Text,
}

impl FieldShape {
    fn admits(self, value: &str, maximum: usize) -> bool {
        !value.trim().is_empty()
            && value.len() <= maximum
            && match self {
                Self::IssueKey => issue_project(value).is_some(),
                Self::NumericId => value.bytes().all(|byte| byte.is_ascii_digit()),
                Self::OneLine => !value.chars().any(char::is_control),
                Self::ProjectKey => {
                    value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                        && value.bytes().all(|byte| {
                            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
                        })
                }
                Self::Text => !value.contains('\0'),
            }
    }
}

pub(super) fn issue_project(value: &str) -> Option<&str> {
    let (project, number) = value.split_once('-')?;
    (!project.is_empty()
        && project.len() <= 32
        && project
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_uppercase)
        && project
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit()))
    .then_some(project)
}

fn supports_operation(connection: &StoredConnection, operation_ref: &str) -> bool {
    is_jira_operation(operation_ref)
        && connection
            .scopes
            .iter()
            .any(|scope| scope == "read:jira-work")
        && (operation_effect(operation_ref) == EffectClass::ReadOnly
            || connection
                .scopes
                .iter()
                .any(|scope| scope == "write:jira-work"))
}

fn operation_effect(operation_ref: &str) -> EffectClass {
    if matches!(
        operation_ref,
        "jira-issue-get" | "jira-issue-comment-list" | "jira-issue-transitions-list"
    ) {
        EffectClass::ReadOnly
    } else {
        EffectClass::Mutating
    }
}

fn operation_approval(operation_ref: &str) -> ApprovalPosture {
    if operation_effect(operation_ref) == EffectClass::ReadOnly {
        ApprovalPosture::NotRequired
    } else {
        ApprovalPosture::Required
    }
}

fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "Jira operation is not granted for this Connection and project",
        false,
    )
}

fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "Jira operation input is invalid",
        false,
    )
}

fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "Jira operation is unavailable",
        true,
    )
}

fn operation_protocol() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "Jira returned an incompatible operation response",
        false,
    )
}

fn operation_outcome_unknown(operation_ref: &str) -> OperationError {
    OperationError::new(
        OperationErrorCode::OutcomeUnknown,
        format!("Jira did not provide a conclusive outcome for {operation_ref}"),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_keys_bind_an_exact_project() {
        assert_eq!(issue_project("OPS-42"), Some("OPS"));
        assert_eq!(issue_project("OPS_TEAM-42"), Some("OPS_TEAM"));
        assert_eq!(issue_project("ops-42"), None);
        assert_eq!(issue_project("12345"), None);
        assert_eq!(issue_project("OPS-other"), None);
    }

    #[test]
    fn all_writes_require_approval() {
        for operation in JIRA_OPERATIONS {
            assert_eq!(
                operation_approval(operation),
                if operation_effect(operation) == EffectClass::ReadOnly {
                    ApprovalPosture::NotRequired
                } else {
                    ApprovalPosture::Required
                }
            );
        }
    }

    #[test]
    fn operation_inputs_are_closed_and_bounded_before_request_assembly() {
        assert!(admit_operation_shape(
            "jira-issue-edit",
            &serde_json::json!({"issue_key":"OPS-42","summary":"Bounded title"}),
        )
        .is_ok());
        assert!(admit_operation_shape(
            "jira-issue-edit",
            &serde_json::json!({"issue_key":"OPS-42","summary":"Bounded title","fields":{}}),
        )
        .is_err());
        assert!(admit_operation_shape(
            "jira-issue-comment-add",
            &serde_json::json!({"issue_key":"OPS-42","body":"x".repeat(32_769)}),
        )
        .is_err());
        assert!(admit_operation_shape(
            "jira-issue-transition",
            &serde_json::json!({"issue_key":"OPS-42","transition_id":"done"}),
        )
        .is_err());
    }

    #[test]
    fn operation_outputs_are_closed_safe_projections() {
        let site = url::Url::parse("https://example.atlassian.net").unwrap();
        let issue = json!({
            "key":"OPS-7","emailAddress":"secret@example.test","expand":"changelog",
            "fields":{
                "summary":"Safe summary","status":{"name":"Open"},
                "issuetype":{"name":"Task"},"priority":null,"assignee":null,"labels":[],
                "updated":"2026-08-17T10:00:00.000+0000","description":"Details",
                "reporter":{"displayName":"Reporter","emailAddress":"secret@example.test"},
                "created":"2026-08-16T10:00:00.000+0000","parent":null,"issuelinks":[],
                "customfield_10001":"SENTINEL-SECRET","attachment":[{"content":"secret"}]
            }
        });
        let projected = project_operation_output(
            "jira-issue-get",
            &issue,
            &site,
            &json!({"issue_key":"OPS-7"}),
        )
        .unwrap();
        let encoded = serde_json::to_string(&projected).unwrap();
        assert!(!encoded.contains("email"));
        assert!(!encoded.contains("customfield"));
        assert!(!encoded.contains("attachment"));
        assert!(!encoded.contains("SENTINEL-SECRET"));
        assert!(
            jsonschema::validator_for(&operation_output_schema("jira-issue-get"))
                .unwrap()
                .is_valid(&projected)
        );

        let comments = project_operation_output(
            "jira-issue-comment-list",
            &json!({
                "startAt":0,"maxResults":50,"total":1,"comments":[{
                    "id":"10","body":"Safe body","created":null,"updated":null,
                    "author":{"displayName":"A Person","emailAddress":"secret@example.test"},
                    "properties":[{"value":"SENTINEL-SECRET"}]
                }]
            }),
            &site,
            &json!({"issue_key":"OPS-7"}),
        )
        .unwrap();
        let encoded = serde_json::to_string(&comments).unwrap();
        assert!(!encoded.contains("email"));
        assert!(!encoded.contains("properties"));
        assert!(!encoded.contains("SENTINEL-SECRET"));
    }
}
