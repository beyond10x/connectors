//! Personal GitLab reads, using the same provider fields and envelope as the hosted adapter.
use super::*;
use serde_json::Map;
fn is_incremental(id: &str) -> bool {
    super::handles(id) && id.starts_with("gitlab-")
}
pub(super) fn input_schema(id: &str) -> Option<Value> {
    if !is_incremental(id) {
        return None;
    }
    let mut properties = serde_json::json!({
        "page":{"type":"integer","minimum":1,"maximum":4294967295_u64},
        "per_page":{"type":"integer","minimum":1,"maximum":100}
    });
    let mut required = vec!["per_page"];
    if id != "gitlab-project-activity-list" {
        properties["project_id"] = serde_json::json!({"type":"integer","minimum":1});
        required.push("project_id");
    }
    let dates: &[&str] = match id {
        "gitlab-project-activity-list" => &["last_activity_after", "last_activity_before"],
        "gitlab-repository-commit-list" => &["since", "until"],
        _ => &["updated_after", "updated_before"],
    };
    for field in dates {
        properties[*field] =
            serde_json::json!({"type":"string","format":"date-time","minLength":1,"maxLength":256});
    }
    if id == "gitlab-repository-commit-list" {
        properties["ref_name"] = serde_json::json!({"type":"string","minLength":1,"maxLength":256});
        required.push("ref_name");
    } else if id == "gitlab-pipeline-list" {
        properties["ref"] = serde_json::json!({"type":"string","minLength":1,"maxLength":256});
    }
    if matches!(id, "gitlab-issue-search" | "gitlab-merge-request-search") {
        properties["state"] = if id == "gitlab-issue-search" {
            serde_json::json!({"type":"string","enum":["opened","closed","all"]})
        } else {
            serde_json::json!({"type":"string","enum":["opened","closed","locked","merged","all"]})
        };
    }
    Some(
        serde_json::json!({"type":"object","additionalProperties":false,"required":required,"properties":properties}),
    )
}

pub(super) fn output_schema(id: &str) -> Option<Value> {
    let fields = fields(id)?;
    let properties: Map<String, Value> = fields
        .iter()
        .map(|field| {
            let schema = match *field {
                "id" if id == "gitlab-repository-commit-list" => {
                    serde_json::json!({"type":"string"})
                }
                "confidential" | "draft" => serde_json::json!({"type":"boolean"}),
                "id" | "iid" | "project_id" => serde_json::json!({"type":"integer"}),
                _ => serde_json::json!({"type":["string","null"]}),
            };
            ((*field).to_owned(), schema)
        })
        .collect();
    Some(serde_json::json!({
        "type":"object","additionalProperties":false,"required":["items","next_page"],
        "properties":{
            "items":{"type":"array","maxItems":100,"items":{
                "type":"object","additionalProperties":false,"required":["id"],"properties":properties
            }},
            "next_page":{"type":["integer","null"],"minimum":1}
        }
    }))
}

fn fields(id: &str) -> Option<&'static [&'static str]> {
    match id {
        "gitlab-issue-search" => Some(&[
            "id",
            "iid",
            "project_id",
            "title",
            "description",
            "state",
            "confidential",
            "created_at",
            "updated_at",
            "closed_at",
            "web_url",
        ]),
        "gitlab-merge-request-search" => Some(&[
            "id",
            "iid",
            "project_id",
            "title",
            "description",
            "state",
            "draft",
            "sha",
            "source_branch",
            "target_branch",
            "created_at",
            "updated_at",
            "merged_at",
            "closed_at",
            "web_url",
        ]),
        "gitlab-project-activity-list" => Some(&[
            "id",
            "name",
            "path_with_namespace",
            "description",
            "default_branch",
            "last_activity_at",
            "web_url",
        ]),
        "gitlab-pipeline-list" => Some(&[
            "id",
            "iid",
            "project_id",
            "sha",
            "ref",
            "status",
            "source",
            "web_url",
            "created_at",
            "updated_at",
        ]),
        "gitlab-deployment-list" => Some(&[
            "id",
            "iid",
            "project_id",
            "sha",
            "ref",
            "status",
            "created_at",
            "updated_at",
            "finished_at",
            "web_url",
            "environment_name",
            "deployable_sha",
        ]),
        "gitlab-repository-commit-list" => Some(&[
            "id",
            "short_id",
            "title",
            "message",
            "created_at",
            "committed_date",
            "web_url",
        ]),
        _ => None,
    }
}

pub(super) fn project(
    id: &str,
    input: &Value,
    response: &EgressHttpResponse,
    payload: &Value,
) -> Result<Value, OperationError> {
    let current = input["page"].as_u64().unwrap_or(1);
    let next = match response.header("x-next-page") {
        Some("") | None => None,
        Some(text) => Some(
            text.parse::<u64>()
                .ok()
                .filter(|n| *n > current && *n <= u32::MAX as u64)
                .ok_or_else(protocol)?,
        ),
    };
    if next.is_none()
        && response
            .header("link")
            .is_some_and(|s| s.contains("rel=\"next\"") || s.contains("rel=next"))
    {
        return Err(protocol());
    }
    let raw = payload.as_array().ok_or_else(protocol)?;
    if raw.len() > input["per_page"].as_u64().ok_or_else(invalid)? as usize {
        return Err(protocol());
    }
    let mut items = Vec::with_capacity(raw.len());
    for source in raw {
        let object = source.as_object().ok_or_else(protocol)?;
        if id != "gitlab-project-activity-list"
            && object
                .get("project_id")
                .is_some_and(|id| id != &input["project_id"])
        {
            return Err(protocol());
        }
        let mut item = Map::new();
        for key in fields(id).ok_or_else(protocol)? {
            if let Some(value) = object.get(*key) {
                if value.as_str().is_some_and(|s| s.len() > 32768) {
                    return Err(protocol());
                }
                item.insert((*key).to_owned(), value.clone());
            }
        }
        if id == "gitlab-deployment-list" {
            for (key, path) in [
                ("environment_name", "/environment/name"),
                ("deployable_sha", "/deployable/commit/id"),
            ] {
                if let Some(value) = source.pointer(path) {
                    item.insert(key.to_owned(), json!(string(value, 1024)?));
                }
            }
            item.insert("project_id".to_owned(), input["project_id"].clone());
        }
        let revision = match id {
            "gitlab-project-activity-list" => "last_activity_at",
            "gitlab-repository-commit-list" => "committed_date",
            _ => "updated_at",
        };
        string(item.get(revision).ok_or_else(protocol)?, 64)?;
        items.push(Value::Object(item));
    }
    let output = json!({"items":items,"next_page":next});
    let schema = output_schema(id).ok_or_else(protocol)?;
    if !jsonschema::validator_for(&schema)
        .map_err(|_| protocol())?
        .is_valid(&output)
    {
        return Err(protocol());
    }
    Ok(output)
}
