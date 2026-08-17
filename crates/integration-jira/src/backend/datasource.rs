//! Bounded Jira issue datasource and allowlist projection.

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine as _};
use protocol::datasource::{
    AccessMode, Completeness, DatasourceBinding, DatasourceDescription, DatasourceError,
    DatasourceErrorCode, DatasourcePage, DatasourceProvenance, DatasourceRead, DatasourceRecord,
    DatasourceRequest, DatasourceResult, DatasourceSummary, ReadRequest, ReadVerb, RecordView,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use service::PrincipalContext;
use sha2::{Digest as _, Sha256};

use super::auth::decode_value_response;
use super::operations::issue_project;
use super::*;

const CURSOR_TTL_MS: u64 = 15 * 60 * 1_000;
const LIST_FIELDS: &str = "summary,status,issuetype,priority,assignee,labels,updated";
const DETAIL_FIELDS: &str = "summary,status,issuetype,priority,assignee,labels,updated,description,reporter,created,parent,issuelinks";

#[derive(Clone)]
struct BindingTarget {
    binding: DatasourceBinding,
    project_key: String,
    user_connection: Option<StoredConnection>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorEnvelope {
    provider_cursor: String,
    issued_at_unix_ms: u64,
    signature_sha256: String,
}

impl JiraInner {
    pub(super) async fn handle_datasource_request(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        match request {
            DatasourceRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let definitions = (query.is_empty()
                    || JIRA_DATASOURCE.contains(&query)
                    || "jira issues".contains(&query))
                .then(datasource_summary)
                .into_iter()
                .collect();
                Ok(DatasourceResult::Search { definitions })
            }
            DatasourceRequest::Describe(request) => self
                .describe_datasource(context, &request.datasource_ref)
                .map(DatasourceResult::Describe),
            DatasourceRequest::Bindings(request) => {
                if request.datasource_ref != JIRA_DATASOURCE {
                    return Err(datasource_not_found());
                }
                let query = request.query.to_ascii_lowercase();
                let mut bindings = self
                    .binding_targets(context)
                    .into_iter()
                    .map(|target| target.binding)
                    .filter(|binding| {
                        query.is_empty() || binding.label.to_ascii_lowercase().contains(&query)
                    })
                    .collect::<Vec<_>>();
                bindings.truncate(usize::from(request.limit));
                Ok(DatasourceResult::Bindings { bindings })
            }
            DatasourceRequest::Read(request) => self
                .read_datasource(context, request)
                .await
                .map(DatasourceResult::Read),
        }
    }

    fn datasource_description_ref(&self, context: &PrincipalContext) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/jira-datasource-description/v1\0");
        digest.update(serde_json::to_vec(context).expect("principal context serializes"));
        digest.update(b"\0");
        digest.update(datasource_projection_sha256().as_bytes());
        digest.update(b"\0");
        digest.update(self.policy.organization_read_grant_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.policy.user_grant_ref.as_bytes());
        for target in self.binding_targets(context) {
            digest.update(b"\0");
            digest.update(target.binding.binding_ref.as_bytes());
            digest.update(target.binding.generation.to_be_bytes());
        }
        format!("datasource-description:jira:{:x}", digest.finalize())
    }

    fn describe_datasource(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
    ) -> Result<DatasourceDescription, DatasourceError> {
        if datasource_ref != JIRA_DATASOURCE {
            return Err(datasource_not_found());
        }
        let (key_schema, compact_schema, detail_schema) = datasource_declaration();
        Ok(DatasourceDescription {
            summary: datasource_summary(),
            description: "Live Jira Cloud issues from one exact admitted project. Organization bindings are read-only for every authenticated tenant member; user bindings remain principal-owned. Raw fields, custom fields, email addresses, comments, attachments, worklogs, changelogs, avatars, and unknown provider members are never projected.".to_owned(),
            key_schema,
            compact_schema,
            detail_schema,
            projection_protocol: VALUE_PROJECTION_PROTOCOL.to_owned(),
            projection_sha256: datasource_projection_sha256(),
            description_ref: self.datasource_description_ref(context),
        })
    }

    fn binding_targets(&self, context: &PrincipalContext) -> Vec<BindingTarget> {
        let mut targets = Vec::new();
        for project in &self.policy.allowed_project_keys {
            targets.push(BindingTarget {
                binding: DatasourceBinding {
                    datasource_ref: JIRA_DATASOURCE.to_owned(),
                    binding_ref: datasource_binding_ref(ORG_CONNECTION_REF, project, 1),
                    connection_ref: ORG_CONNECTION_REF.to_owned(),
                    label: format!("{project} · organization read-only"),
                    generation: 1,
                },
                project_key: project.clone(),
                user_connection: None,
            });
        }
        for connection in self.owned_user_connections(context) {
            for project in &self.policy.allowed_project_keys {
                targets.push(BindingTarget {
                    binding: DatasourceBinding {
                        datasource_ref: JIRA_DATASOURCE.to_owned(),
                        binding_ref: datasource_binding_ref(
                            &connection.connection_ref,
                            project,
                            connection.credential_generation,
                        ),
                        connection_ref: connection.connection_ref.clone(),
                        label: format!("{project} · {}", connection.label),
                        generation: connection.credential_generation,
                    },
                    project_key: project.clone(),
                    user_connection: Some(connection.clone()),
                });
            }
        }
        targets.sort_by(|left, right| left.binding.binding_ref.cmp(&right.binding.binding_ref));
        targets
    }

    async fn read_datasource(
        &self,
        context: &PrincipalContext,
        request: ReadRequest,
    ) -> Result<DatasourcePage, DatasourceError> {
        if request.datasource_ref != JIRA_DATASOURCE {
            return Err(datasource_not_found());
        }
        if request.description_ref != self.datasource_description_ref(context) {
            return Err(DatasourceError::new(
                DatasourceErrorCode::StaleAuthority,
                "Jira datasource description lease is stale",
                false,
            ));
        }
        let target = self
            .binding_targets(context)
            .into_iter()
            .find(|target| target.binding.binding_ref == request.binding_ref)
            .ok_or_else(datasource_not_granted)?;
        let token = match target.user_connection.as_ref() {
            Some(connection) => self.user_access_token(connection).await,
            None => self.service_access_token().await,
        }
        .map_err(|_| datasource_unavailable())?;
        let (url, view, provider_cursor, requested_limit, expected_issue_key) = match &request.read
        {
            DatasourceRead::List { limit, cursor } => {
                let provider_cursor = cursor
                    .as_deref()
                    .map(|cursor| {
                        self.parse_cursor(cursor, context, &target.binding, &target.project_key)
                    })
                    .transpose()?;
                let mut url = self.gateway_url("rest/api/2/search/jql")?;
                let jql = format!(
                    "project = \"{}\" ORDER BY updated DESC, key DESC",
                    target.project_key
                );
                url.query_pairs_mut()
                    .append_pair("jql", &jql)
                    .append_pair("fields", LIST_FIELDS)
                    .append_pair("maxResults", &limit.saturating_add(1).to_string());
                if let Some(cursor) = provider_cursor.as_deref() {
                    url.query_pairs_mut().append_pair("nextPageToken", cursor);
                }
                (
                    url,
                    RecordView::Compact,
                    provider_cursor,
                    usize::from(*limit),
                    None,
                )
            }
            DatasourceRead::Get { key } => {
                let issue_key = key.as_str().ok_or_else(datasource_invalid)?;
                if issue_project(issue_key) != Some(target.project_key.as_str()) {
                    return Err(datasource_not_granted());
                }
                let mut url = self.gateway_url(&format!("rest/api/2/issue/{issue_key}"))?;
                url.query_pairs_mut().append_pair("fields", DETAIL_FIELDS);
                (url, RecordView::Detail, None, 1, Some(issue_key.to_owned()))
            }
        };
        let audit_ref = format!(
            "audit:jira:{}",
            random_uuid().map_err(|_| datasource_unavailable())?
        );
        self.audit(
            &audit_ref,
            match request.read {
                DatasourceRead::List { .. } => "jira.issues.list",
                DatasourceRead::Get { .. } => "jira.issues.get",
            },
            &target.binding.connection_ref,
            context,
            "attempted",
        )
        .map_err(|_| datasource_unavailable())?;
        let organization_read = target.user_connection.is_none();
        let response = self
            .http
            .get(url)
            .bearer_auth(token.expose_secret())
            .header("Accept", "application/json")
            .send()
            .await;
        drop(token);
        if organization_read && response.is_err() {
            self.service_callable.store(false, Ordering::Release);
        }
        let response = response.map_err(|_| datasource_unavailable())?;
        if !response.status().is_success() {
            if organization_read {
                self.service_callable.store(false, Ordering::Release);
            }
            return Err(match response.status() {
                reqwest::StatusCode::NOT_FOUND => datasource_not_found(),
                reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
                    datasource_not_granted()
                }
                _ => datasource_unavailable(),
            });
        }
        if organization_read {
            self.service_callable.store(true, Ordering::Release);
        }
        let payload = decode_value_response(response, protocol::datasource::MAX_RESULT_BYTES)
            .await
            .map_err(|_| datasource_unavailable())?;
        let (records, next_provider_cursor) = match view {
            RecordView::Compact => {
                let object = payload.as_object().ok_or_else(datasource_protocol)?;
                let values = object
                    .get("issues")
                    .and_then(Value::as_array)
                    .ok_or_else(datasource_protocol)?;
                let has_more = values.len() > requested_limit
                    || object.get("isLast").and_then(Value::as_bool) == Some(false);
                let records = values
                    .iter()
                    .take(requested_limit)
                    .map(|value| project_issue(value, RecordView::Compact, &self.site_origin))
                    .collect::<Result<Vec<_>, _>>()?;
                let next = has_more
                    .then(|| {
                        object
                            .get("nextPageToken")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .flatten();
                (records, next)
            }
            RecordView::Detail => (
                vec![project_issue(
                    &payload,
                    RecordView::Detail,
                    &self.site_origin,
                )?],
                None,
            ),
        };
        if records.iter().any(|record| {
            let key = record.key.as_str();
            key.and_then(issue_project) != Some(target.project_key.as_str())
                || expected_issue_key
                    .as_deref()
                    .is_some_and(|expected| key != Some(expected))
        }) {
            return Err(datasource_protocol());
        }
        let description = self.describe_datasource(context, JIRA_DATASOURCE)?;
        let schema = match view {
            RecordView::Compact => &description.compact_schema,
            RecordView::Detail => &description.detail_schema,
        };
        let validator = jsonschema::validator_for(schema).map_err(|_| datasource_unavailable())?;
        if records
            .iter()
            .any(|record| !validator.is_valid(&record.value))
        {
            return Err(datasource_protocol());
        }
        self.audit(
            &audit_ref,
            if view == RecordView::Compact {
                "jira.issues.list"
            } else {
                "jira.issues.get"
            },
            &target.binding.connection_ref,
            context,
            "completed",
        )
        .map_err(|_| datasource_unavailable())?;
        let next_cursor = next_provider_cursor
            .map(|provider| self.cursor(&provider, context, &target.binding, &target.project_key))
            .transpose()?;
        let _ = provider_cursor;
        Ok(DatasourcePage {
            datasource_ref: JIRA_DATASOURCE.to_owned(),
            records,
            next_cursor,
            completeness: Completeness::Complete,
            observed_at_unix_ms: now_ms().ok_or_else(datasource_unavailable)?,
            provenance: DatasourceProvenance {
                binding_ref: target.binding.binding_ref,
                projection_sha256: description.projection_sha256,
                connector_audit_ref: audit_ref,
            },
        })
    }

    fn gateway_url(&self, path: &str) -> Result<url::Url, DatasourceError> {
        if path.starts_with('/') || path.contains("..") || path.contains("//") {
            return Err(datasource_invalid());
        }
        let target = self
            .gateway_origin
            .join(path)
            .map_err(|_| datasource_unavailable())?;
        self.admitted_gateway_target(&target)
            .then_some(target)
            .ok_or_else(datasource_not_granted)
    }

    fn cursor(
        &self,
        provider_cursor: &str,
        context: &PrincipalContext,
        binding: &DatasourceBinding,
        project: &str,
    ) -> Result<String, DatasourceError> {
        if provider_cursor.is_empty() || provider_cursor.len() > 2_048 {
            return Err(datasource_protocol());
        }
        let issued_at_unix_ms = now_ms().ok_or_else(datasource_unavailable)?;
        let signature_sha256 = self.cursor_signature(
            provider_cursor,
            issued_at_unix_ms,
            context,
            binding,
            project,
        );
        let bytes = serde_json::to_vec(&CursorEnvelope {
            provider_cursor: provider_cursor.to_owned(),
            issued_at_unix_ms,
            signature_sha256,
        })
        .map_err(|_| datasource_unavailable())?;
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }

    fn parse_cursor(
        &self,
        cursor: &str,
        context: &PrincipalContext,
        binding: &DatasourceBinding,
        project: &str,
    ) -> Result<String, DatasourceError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|_| datasource_cursor_expired())?;
        let envelope: CursorEnvelope =
            serde_json::from_slice(&bytes).map_err(|_| datasource_cursor_expired())?;
        let now = now_ms().ok_or_else(datasource_unavailable)?;
        if envelope.provider_cursor.is_empty()
            || envelope.provider_cursor.len() > 2_048
            || envelope.issued_at_unix_ms > now
            || now.saturating_sub(envelope.issued_at_unix_ms) > CURSOR_TTL_MS
            || envelope.signature_sha256
                != self.cursor_signature(
                    &envelope.provider_cursor,
                    envelope.issued_at_unix_ms,
                    context,
                    binding,
                    project,
                )
        {
            return Err(datasource_cursor_expired());
        }
        Ok(envelope.provider_cursor)
    }

    fn cursor_signature(
        &self,
        provider_cursor: &str,
        issued_at_unix_ms: u64,
        context: &PrincipalContext,
        binding: &DatasourceBinding,
        project: &str,
    ) -> String {
        let mut digest = Sha256::new();
        digest.update(self.cursor_key);
        digest.update(b"\0b10x/jira-datasource-cursor/v1\0");
        digest.update(context.tenant_id().as_bytes());
        digest.update(b"\0");
        digest.update(context.subject().as_bytes());
        digest.update(b"\0");
        digest.update(binding.binding_ref.as_bytes());
        digest.update(binding.generation.to_be_bytes());
        digest.update(b"\0");
        digest.update(project.as_bytes());
        digest.update(b"\0");
        digest.update(issued_at_unix_ms.to_be_bytes());
        digest.update(provider_cursor.as_bytes());
        hex::encode(digest.finalize())
    }
}

pub(super) fn project_issue(
    value: &Value,
    view: RecordView,
    site_origin: &url::Url,
) -> Result<DatasourceRecord, DatasourceError> {
    let object = value.as_object().ok_or_else(datasource_protocol)?;
    let key = bounded_required(object.get("key"), 64)?;
    if issue_project(&key).is_none() {
        return Err(datasource_protocol());
    }
    let fields = object
        .get("fields")
        .and_then(Value::as_object)
        .ok_or_else(datasource_protocol)?;
    let mut projected = Map::new();
    projected.insert("key".to_owned(), Value::String(key.clone()));
    projected.insert(
        "summary".to_owned(),
        Value::String(bounded_required(fields.get("summary"), 512)?),
    );
    insert_nested_name(&mut projected, "status", fields.get("status"), 128)?;
    insert_nested_name(&mut projected, "type", fields.get("issuetype"), 128)?;
    insert_optional_nested_name(&mut projected, "priority", fields.get("priority"), 128)?;
    insert_optional_nested_name(&mut projected, "assignee", fields.get("assignee"), 256)?;
    let labels = fields
        .get("labels")
        .and_then(Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .take(32)
                .filter_map(Value::as_str)
                .map(|label| Value::String(bounded_string(label, 128)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    projected.insert("labels".to_owned(), Value::Array(labels));
    projected.insert(
        "updated".to_owned(),
        Value::String(bounded_required(fields.get("updated"), 64)?),
    );
    projected.insert(
        "browser_url".to_owned(),
        Value::String(format!(
            "{}/browse/{key}",
            site_origin.as_str().trim_end_matches('/')
        )),
    );
    if view == RecordView::Detail {
        insert_optional_string(
            &mut projected,
            "description",
            fields.get("description"),
            32_768,
        )?;
        insert_optional_nested_name(&mut projected, "reporter", fields.get("reporter"), 256)?;
        insert_optional_string(&mut projected, "created", fields.get("created"), 64)?;
        let parent = fields
            .get("parent")
            .and_then(Value::as_object)
            .and_then(|parent| parent.get("key"))
            .and_then(Value::as_str)
            .map(|parent| Value::String(bounded_string(parent, 64)))
            .unwrap_or(Value::Null);
        projected.insert("parent_key".to_owned(), parent);
        projected.insert(
            "links".to_owned(),
            Value::Array(project_links(fields.get("issuelinks"))?),
        );
    }
    Ok(DatasourceRecord {
        key: Value::String(key),
        view,
        value: Value::Object(projected),
    })
}

fn project_links(value: Option<&Value>) -> Result<Vec<Value>, DatasourceError> {
    let Some(links) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(links
        .iter()
        .take(32)
        .filter_map(|link| {
            let object = link.as_object()?;
            let link_type = object.get("type")?.as_object()?.get("name")?.as_str()?;
            let (direction, issue) = object
                .get("inwardIssue")
                .map(|issue| ("inward", issue))
                .or_else(|| object.get("outwardIssue").map(|issue| ("outward", issue)))?;
            let issue = issue.as_object()?;
            let key = issue.get("key")?.as_str()?;
            let summary = issue.get("fields")?.as_object()?.get("summary")?.as_str()?;
            Some(json!({
                "direction": direction,
                "type": bounded_string(link_type, 128),
                "issue_key": bounded_string(key, 64),
                "summary": bounded_string(summary, 512),
            }))
        })
        .collect::<Vec<_>>())
}

fn bounded_required(value: Option<&Value>, maximum: usize) -> Result<String, DatasourceError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|value| bounded_string(value, maximum))
        .ok_or_else(datasource_protocol)
}

fn insert_nested_name(
    output: &mut Map<String, Value>,
    key: &str,
    value: Option<&Value>,
    maximum: usize,
) -> Result<(), DatasourceError> {
    let name = value
        .and_then(Value::as_object)
        .and_then(|object| object.get("name"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(datasource_protocol)?;
    output.insert(key.to_owned(), Value::String(bounded_string(name, maximum)));
    Ok(())
}

fn insert_optional_nested_name(
    output: &mut Map<String, Value>,
    key: &str,
    value: Option<&Value>,
    maximum: usize,
) -> Result<(), DatasourceError> {
    let projected = match value {
        None | Some(Value::Null) => Value::Null,
        Some(value) => {
            let name = value
                .as_object()
                .and_then(|object| object.get("displayName").or_else(|| object.get("name")))
                .and_then(Value::as_str)
                .ok_or_else(datasource_protocol)?;
            Value::String(bounded_string(name, maximum))
        }
    };
    output.insert(key.to_owned(), projected);
    Ok(())
}

fn insert_optional_string(
    output: &mut Map<String, Value>,
    key: &str,
    value: Option<&Value>,
    maximum: usize,
) -> Result<(), DatasourceError> {
    let projected = match value {
        None | Some(Value::Null) => Value::Null,
        Some(value) => Value::String(bounded_string(
            value.as_str().ok_or_else(datasource_protocol)?,
            maximum,
        )),
    };
    output.insert(key.to_owned(), projected);
    Ok(())
}

fn datasource_binding_ref(connection_ref: &str, project: &str, generation: u64) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/jira-datasource-binding/v1\0");
    digest.update(connection_ref.as_bytes());
    digest.update(b"\0");
    digest.update(project.as_bytes());
    digest.update(generation.to_be_bytes());
    format!(
        "datasource-binding:jira:issues:{}:{:x}",
        project,
        digest.finalize()
    )
}

fn datasource_summary() -> DatasourceSummary {
    DatasourceSummary {
        datasource_ref: JIRA_DATASOURCE.to_owned(),
        title: "Jira issues".to_owned(),
        access_mode: AccessMode::Live,
        verbs: vec![ReadVerb::List, ReadVerb::Get],
    }
}

pub(super) fn datasource_declaration() -> (Value, Value, Value) {
    let common = json!({
        "key": {"type":"string","minLength":3,"maxLength":64},
        "summary": {"type":"string","maxLength":512},
        "status": {"type":"string","maxLength":128},
        "type": {"type":"string","maxLength":128},
        "priority": {"type":["string","null"],"maxLength":128},
        "assignee": {"type":["string","null"],"maxLength":256},
        "labels": {"type":"array","maxItems":32,"items":{"type":"string","maxLength":128}},
        "updated": {"type":"string","maxLength":64},
        "browser_url": {"type":"string","format":"uri","maxLength":2048}
    });
    let mut detail = common.as_object().expect("object").clone();
    detail.insert(
        "description".to_owned(),
        json!({"type":["string","null"],"maxLength":32768}),
    );
    detail.insert(
        "reporter".to_owned(),
        json!({"type":["string","null"],"maxLength":256}),
    );
    detail.insert(
        "created".to_owned(),
        json!({"type":["string","null"],"maxLength":64}),
    );
    detail.insert(
        "parent_key".to_owned(),
        json!({"type":["string","null"],"maxLength":64}),
    );
    detail.insert("links".to_owned(), json!({
        "type":"array","maxItems":32,"items":{"type":"object","additionalProperties":false,
        "required":["direction","type","issue_key","summary"],"properties":{
            "direction":{"type":"string","enum":["inward","outward"]},
            "type":{"type":"string","maxLength":128},"issue_key":{"type":"string","maxLength":64},
            "summary":{"type":"string","maxLength":512}}}
    }));
    (
        json!({"type":"string","minLength":3,"maxLength":64}),
        json!({
            "type":"object","additionalProperties":false,
            "required":["key","summary","status","type","priority","assignee","labels","updated","browser_url"],
            "properties": common
        }),
        json!({
            "type":"object","additionalProperties":false,
            "required":["key","summary","status","type","priority","assignee","labels","updated","browser_url","description","reporter","created","parent_key","links"],
            "properties": Value::Object(detail)
        }),
    )
}

fn datasource_projection_sha256() -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/jira-issues-projection/v1\0");
    let (key, compact, detail) = datasource_declaration();
    for value in [key, compact, detail] {
        digest.update(serde_json::to_vec(&value).expect("schema serializes"));
        digest.update(b"\0");
    }
    hex::encode(digest.finalize())
}

fn datasource_not_found() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::NotFound,
        "Jira datasource or issue was not found",
        false,
    )
}

fn datasource_not_granted() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::NotGranted,
        "Jira datasource is not granted for this Connection and project",
        false,
    )
}

fn datasource_invalid() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::InvalidInput,
        "Jira datasource input is invalid",
        false,
    )
}

fn datasource_protocol() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::Protocol,
        "Jira returned an incompatible datasource response",
        false,
    )
}

fn datasource_unavailable() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::Unavailable,
        "Jira datasource is unavailable",
        true,
    )
}

fn datasource_cursor_expired() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::CursorExpired,
        "Jira datasource cursor is stale or belongs to another binding",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_drops_sensitive_and_unknown_provider_fields() {
        let issue = json!({
            "id":"10001","key":"OPS-7","expand":"changelog","emailAddress":"secret@example.test",
            "fields":{
                "summary":"Safe summary","status":{"name":"Open"},"issuetype":{"name":"Task"},
                "priority":{"name":"High"},"assignee":{"displayName":"A Person","emailAddress":"secret@example.test"},
                "labels":["safe"],"updated":"2026-08-17T10:00:00.000+0000","description":"Details",
                "reporter":{"displayName":"Reporter","emailAddress":"reporter@example.test"},
                "created":"2026-08-16T10:00:00.000+0000","customfield_10001":"SENTINEL-SECRET",
                "attachment":[{"content":"SENTINEL-SECRET"}],"worklog":{"worklogs":[]},
                "issuelinks":[]
            }
        });
        let site = url::Url::parse("https://example.atlassian.net").unwrap();
        let projected = project_issue(&issue, RecordView::Detail, &site).unwrap();
        let encoded = serde_json::to_string(&projected.value).unwrap();
        assert!(!encoded.contains("email"));
        assert!(!encoded.contains("customfield"));
        assert!(!encoded.contains("attachment"));
        assert!(!encoded.contains("worklog"));
        assert!(!encoded.contains("SENTINEL-SECRET"));
    }

    #[test]
    fn schemas_are_closed_and_projection_is_stable() {
        let (_, compact, detail) = datasource_declaration();
        assert_eq!(compact["additionalProperties"], false);
        assert_eq!(detail["additionalProperties"], false);
        assert_eq!(datasource_projection_sha256().len(), 64);
    }
}
