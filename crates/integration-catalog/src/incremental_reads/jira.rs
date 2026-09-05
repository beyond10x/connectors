//! Personal Jira reads use trusted resolved request URLs, never provider-supplied origins.
use super::*;
fn is_incremental(id: &str) -> bool {
    matches!(
        id,
        "jira-issue-search" | "jira-project-list" | "jira-issue-comments-read"
    )
}
pub(super) fn input_schema(id: &str) -> Option<Value> {
    if !is_incremental(id) {
        return None;
    }
    let mut properties = json!({"limit":{"type":"integer","minimum":1,"maximum":100}});
    let required = if id == "jira-issue-search" {
        properties["project_key"] = json!({"type":"string","pattern":"^[A-Z][A-Z0-9_]{0,31}$","minLength":1,"maxLength":32});
        properties["updated_since_ms"] =
            json!({"type":"integer","minimum":0,"maximum":253402300799999_u64});
        properties["next_page_token"] = json!({"type":"string","minLength":1,"maxLength":4096});
        vec!["project_key", "updated_since_ms", "limit"]
    } else if id == "jira-issue-comments-read" {
        properties["issue_key"] = json!({"type":"string","pattern":"^[A-Z][A-Z0-9_]{0,31}-[0-9]+$","minLength":3,"maxLength":64});
        properties["start_at"] = json!({"type":"integer","minimum":0,"maximum":4294967295_u64});
        vec!["issue_key", "limit"]
    } else {
        properties["start_at"] = json!({"type":"integer","minimum":0,"maximum":4294967295_u64});
        vec!["limit"]
    };
    Some(
        json!({"type":"object","additionalProperties":false,"properties":properties,"required":required}),
    )
}

pub(super) fn prepare_request(
    input: &Value,
    request: &mut connector_resolve::Request,
) -> Result<(), OperationError> {
    let mut target = url::Url::parse(&request.url).map_err(|_| invalid())?;
    let project = string(&input["project_key"], 32)?;
    let since = input["updated_since_ms"].as_u64().ok_or_else(invalid)?;
    target.set_query(None);
    target
        .query_pairs_mut()
        .append_pair(
            "jql",
            &format!(
                "project = \"{project}\" AND updated >= {since} ORDER BY updated ASC, key ASC"
            ),
        )
        .append_pair(
            "fields",
            "summary,status,issuetype,priority,labels,updated,description,created",
        )
        .append_pair(
            "maxResults",
            &input["limit"].as_u64().ok_or_else(invalid)?.to_string(),
        );
    if let Some(cursor) = input["next_page_token"].as_str() {
        target
            .query_pairs_mut()
            .append_pair("nextPageToken", cursor);
    }
    request.url = target.into();
    Ok(())
}

pub(super) fn project(
    id: &str,
    input: &Value,
    request_url: &str,
    payload: &Value,
) -> Result<Value, OperationError> {
    if id == "jira-issue-comments-read" {
        return super::comments::project(input, payload);
    }
    let last = payload["isLast"].as_bool().ok_or_else(protocol)?;
    let limit = input["limit"].as_u64().ok_or_else(invalid)? as usize;
    if id == "jira-project-list" {
        let raw = payload["values"].as_array().ok_or_else(protocol)?;
        let start = payload["startAt"].as_u64().ok_or_else(protocol)?;
        let size = payload["maxResults"]
            .as_u64()
            .filter(|v| *v <= 100)
            .ok_or_else(protocol)?;
        let total = payload["total"].as_u64().ok_or_else(protocol)?;
        if raw.len() > limit
            || start != input["start_at"].as_u64().unwrap_or(0)
            || (!last && (size == 0 || raw.is_empty()))
        {
            return Err(protocol());
        }
        let values=raw.iter().map(|v|Ok(json!({"id":string(&v["id"],64)?,"key":string(&v["key"],32)?,"name":string(&v["name"],256)?}))).collect::<Result<Vec<_>,OperationError>>()?;
        let next = if last {
            None
        } else {
            Some(
                start
                    .checked_add(size)
                    .filter(|n| *n <= u32::MAX as u64)
                    .ok_or_else(protocol)?,
            )
        };
        return Ok(
            json!({"values":values,"is_last":last,"start_at":start,"max_results":size,"total":total,"next_start_at":next}),
        );
    }
    let raw = payload["issues"].as_array().ok_or_else(protocol)?;
    if raw.len() > limit {
        return Err(protocol());
    }
    let mut issues = Vec::with_capacity(raw.len());
    for source in raw {
        let id = string(&source["id"], 64)?;
        let key = string(&source["key"], 64)?;
        let (project, number) = key.split_once('-').ok_or_else(protocol)?;
        if Some(project) != input["project_key"].as_str()
            || number.is_empty()
            || !number.bytes().all(|c| c.is_ascii_digit())
        {
            return Err(protocol());
        }
        let fields = &source["fields"];
        // The trusted configured gateway can supply an API citation, not an inferred tenant UI.
        // The caller's separately trusted company record supplies a human-facing UI link.
        let mut api = url::Url::parse(request_url).map_err(|_| protocol())?;
        api.set_query(None);
        let path = api
            .path()
            .strip_suffix("/search/jql")
            .ok_or_else(protocol)?
            .to_owned();
        api.set_path(&format!("{path}/issue/{key}"));
        let optional = |key: &str, max: usize| -> Result<Value, OperationError> {
            match fields.get(key) {
                None | Some(Value::Null) => Ok(Value::Null),
                Some(Value::String(value)) if value.len() <= max => Ok(json!(value)),
                Some(_) => Err(protocol()),
            }
        };
        let labels = match fields.get("labels") {
            None => Vec::new(),
            Some(value) => {
                let values = value
                    .as_array()
                    .filter(|v| v.len() <= 32)
                    .ok_or_else(protocol)?;
                values
                    .iter()
                    .map(|v| string(v, 128))
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        issues.push(json!({"id":id,"key":key,"summary":string(&fields["summary"],512)?,"status":string(&fields["status"]["name"],128)?,
            "type":string(&fields["issuetype"]["name"],128)?,"updated":string(&fields["updated"],64)?,"description":optional("description",32768)?,
            "created":optional("created",64)?,"priority":fields.get("priority").filter(|v|!v.is_null()).map(|v|string(&v["name"],128)).transpose()?,
            "assignee":null,"reporter":null,"parent_key":null,"links":[],"labels":labels,"browser_url":null,"api_url":api.as_str()}));
    }
    let next = if last {
        Value::Null
    } else {
        let cursor = string(&payload["nextPageToken"], 4096)?;
        if Some(cursor.as_str()) == input["next_page_token"].as_str() {
            return Err(protocol());
        }
        json!(cursor)
    };
    Ok(json!({"issues":issues,"is_last":last,"next_page_token":next}))
}
