//! Bounded activity pages. Keep vendor pagination visible without projecting user objects.
use super::*;

pub(super) fn is_incremental(id: &str) -> bool {
    matches!(
        id,
        "gitlab-project-activity-list"
            | "gitlab-pipeline-list"
            | "gitlab-deployment-list"
            | "gitlab-repository-commit-list"
    )
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

fn protocol_error() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "GitLab activity response violates its bounded continuation contract",
        false,
    )
}

pub(super) fn decode(
    id: &str,
    input: &Value,
    response: EgressHttpResponse,
) -> Result<Value, OperationError> {
    if matches!(response.status, 401 | 403 | 404) {
        return Err(operation_not_granted());
    }
    if !response.is_success() {
        return Err(operation_unavailable());
    }
    let current = input["page"].as_u64().unwrap_or(1);
    let next = match response.header("x-next-page") {
        Some("") => None,
        Some(value) => Some(
            value
                .parse::<u64>()
                .ok()
                .filter(|next| *next > current)
                .ok_or_else(protocol_error)?,
        ),
        None => {
            // A Link-only continuation must never masquerade as a completed page.
            if response
                .header("link")
                .is_some_and(|value| value.contains("rel=\"next\"") || value.contains("rel=next"))
            {
                return Err(protocol_error());
            }
            None
        }
    };
    let payload = decode_value_response(response).map_err(|_| protocol_error())?;
    let items = payload.as_array().ok_or_else(protocol_error)?;
    let limit = input["per_page"].as_u64().ok_or_else(operation_invalid)?;
    if items.len() > limit as usize {
        return Err(protocol_error());
    }
    let allowed = fields(id).ok_or_else(operation_not_found)?;
    let mut projected = Vec::with_capacity(items.len());
    for value in items {
        let object = value.as_object().ok_or_else(protocol_error)?;
        if object
            .get("project_id")
            .is_some_and(|value| value != &input["project_id"])
        {
            return Err(protocol_error());
        }
        let mut item = Map::new();
        for key in allowed {
            if let Some(value) = object.get(*key) {
                if !(value.is_null()
                    || value.is_number()
                    || value.as_str().is_some_and(|s| s.len() <= 32768))
                {
                    return Err(protocol_error());
                }
                item.insert((*key).to_owned(), value.clone());
            }
        }
        if id == "gitlab-deployment-list" {
            for (key, pointer) in [
                ("environment_name", "/environment/name"),
                ("deployable_sha", "/deployable/commit/id"),
            ] {
                if let Some(value) = value.pointer(pointer).and_then(Value::as_str) {
                    if value.len() > 1024 {
                        return Err(protocol_error());
                    }
                    item.insert(key.to_owned(), Value::String(value.to_owned()));
                }
            }
            item.insert("project_id".to_owned(), input["project_id"].clone());
        }
        let revision = if id == "gitlab-project-activity-list" {
            "last_activity_at"
        } else if id == "gitlab-repository-commit-list" {
            "committed_date"
        } else {
            "updated_at"
        };
        // The vendor list must supply revision evidence: missing timestamps are an error, not a fresh observation.
        if !item.get(revision).is_some_and(Value::is_string) {
            return Err(protocol_error());
        }
        projected.push(Value::Object(item));
    }
    let result = serde_json::json!({"items":projected,"next_page":next});
    let schema = output_schema(id).ok_or_else(operation_not_found)?;
    if !jsonschema::validator_for(&schema)
        .map_err(|_| protocol_error())?
        .is_valid(&result)
    {
        return Err(protocol_error());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn incremental_inputs_preserve_integer_bounds_and_closed_fields() {
        let schema = input_schema("gitlab-pipeline-list").unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&json!({"project_id":7,"per_page":2})));
        for invalid in [
            json!({"project_id":7,"per_page":101}),
            json!({"project_id":7,"per_page":0.5}),
            json!({"project_id":7,"per_page":2,"unexpected":"value"}),
            json!({"project_id":-1,"per_page":2}),
        ] {
            assert!(
                !validator.is_valid(&invalid),
                "invalid incremental input admitted: {invalid}"
            );
        }
    }

    fn response(items: Value, next: &str) -> EgressHttpResponse {
        EgressHttpResponse {
            status: 200,
            headers: BTreeMap::from([("x-next-page".to_owned(), next.to_owned())]),
            body: serde_json::to_vec(&items).unwrap(),
        }
    }

    #[test]
    fn two_pages_preserve_overlap_revisions_and_scrub_unknown_members() {
        for id in [
            "gitlab-project-activity-list",
            "gitlab-pipeline-list",
            "gitlab-deployment-list",
            "gitlab-repository-commit-list",
        ] {
            let record = if id == "gitlab-project-activity-list" {
                json!({"id":7,"name":"Project","path_with_namespace":"group/project","default_branch":"main","last_activity_at":"2026-09-05T10:00:00Z","web_url":"https://example.test/group/project","owner":{"email":"private@example.test"}})
            } else if id == "gitlab-repository-commit-list" {
                json!({"id":"abc123","committed_date":"2026-09-05T10:00:00Z","author_email":"private@example.test"})
            } else {
                json!({"id":1,"project_id":7,"updated_at":"2026-09-05T10:00:00Z","user":{"email":"private@example.test"},"unknown":"sentinel"})
            };
            let first = decode(
                id,
                &json!({"project_id":7,"per_page":2,"page":1}),
                response(json!([record.clone()]), "2"),
            )
            .unwrap();
            let last = decode(
                id,
                &json!({"project_id":7,"per_page":2,"page":2}),
                response(json!([record]), ""),
            )
            .unwrap();
            assert_eq!(first["next_page"], 2);
            assert!(last["next_page"].is_null());
            assert_eq!(first["items"][0]["id"], last["items"][0]["id"]);
            assert!(!first.to_string().contains("private"));
            assert!(!first.to_string().contains("sentinel"));
        }
    }

    #[test]
    fn malformed_continuations_permissions_and_foreign_projects_fail_closed() {
        let input = json!({"project_id":7,"per_page":2,"page":2});
        let record = json!({"id":1,"project_id":8,"updated_at":"2026-09-05T10:00:00Z"});
        assert!(
            decode(
                "gitlab-pipeline-list",
                &input,
                response(json!([record]), "")
            )
            .is_err(),
            "a response from another project must be refused"
        );
        for cursor in ["2", "1", "invalid", "0"] {
            assert!(decode("gitlab-pipeline-list", &input, response(json!([]), cursor)).is_err());
        }
        for status in [401, 403, 404, 429] {
            let error = decode(
                "gitlab-pipeline-list",
                &input,
                EgressHttpResponse {
                    status,
                    headers: BTreeMap::new(),
                    body: b"private provider error".to_vec(),
                },
            )
            .unwrap_err();
            assert_eq!(
                error.code,
                if status == 429 {
                    OperationErrorCode::Unavailable
                } else {
                    OperationErrorCode::NotGranted
                }
            );
            assert!(!error.message.contains("private"));
        }
    }
}
