//! Native incremental reads share Jira custody, admission, request execution and issue projection.
use super::*;

pub(super) fn is_incremental(id: &str) -> bool {
    matches!(id, "jira-issue-search" | "jira-project-list")
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
    } else {
        properties["start_at"] = json!({"type":"integer","minimum":0,"maximum":4294967295_u64});
        vec!["limit"]
    };
    Some(
        json!({"type":"object","additionalProperties":false,"properties":properties,"required":required}),
    )
}

pub(super) fn admit(id: &str, input: &Value) -> Result<(), OperationError> {
    let object = input.as_object().ok_or_else(operation_invalid)?;
    let allowed: &[&str] = if id == "jira-issue-search" {
        &[
            "project_key",
            "updated_since_ms",
            "limit",
            "next_page_token",
        ]
    } else {
        &["limit", "start_at"]
    };
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || !input["limit"]
            .as_u64()
            .is_some_and(|limit| (1..=100).contains(&limit))
    {
        return Err(operation_invalid());
    }
    if id == "jira-issue-search" {
        if !input["project_key"]
            .as_str()
            .is_some_and(|key| FieldShape::ProjectKey.admits(key, 32))
            || !input["updated_since_ms"]
                .as_u64()
                .is_some_and(|value| value <= 253402300799999)
            || object.get("next_page_token").is_some_and(|value| {
                !value.as_str().is_some_and(|s| {
                    !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
                })
            })
        {
            return Err(operation_invalid());
        }
    } else if object
        .get("start_at")
        .is_some_and(|value| !value.as_u64().is_some_and(|v| v <= u32::MAX as u64))
    {
        return Err(operation_invalid());
    }
    Ok(())
}

pub(super) fn prepare_query(
    id: &str,
    input: &Value,
    target: &mut url::Url,
) -> Result<(), OperationError> {
    if id != "jira-issue-search" {
        return Ok(());
    }
    admit(id, input)?;
    let project = input["project_key"]
        .as_str()
        .ok_or_else(operation_invalid)?;
    let since = input["updated_since_ms"]
        .as_u64()
        .ok_or_else(operation_invalid)?;
    let limit = input["limit"].as_u64().ok_or_else(operation_invalid)?;
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
        .append_pair("maxResults", &limit.to_string());
    if let Some(token) = input["next_page_token"].as_str() {
        target.query_pairs_mut().append_pair("nextPageToken", token);
    }
    Ok(())
}

pub(super) fn output_schema(id: &str) -> Option<Value> {
    let integer = json!({"type":"integer","minimum":0});
    match id {
        "jira-issue-search" => {
            let mut issue = super::super::datasource::datasource_declaration().2;
            issue["properties"]["id"] = json!({"type":"string","minLength":1,"maxLength":64});
            issue["required"].as_array_mut()?.push(json!("id"));
            Some(
                json!({"type":"object","additionalProperties":false,"required":["issues","is_last","next_page_token"],"properties":{
                    "issues":{"type":"array","maxItems":100,"items":issue},"is_last":{"type":"boolean"},
                    "next_page_token":{"type":["string","null"],"minLength":1,"maxLength":4096}
                }}),
            )
        }
        "jira-project-list" => Some(json!({"type":"object","additionalProperties":false,
        "required":["values","is_last","start_at","max_results","total","next_start_at"],"properties":{
            "values":{"type":"array","maxItems":100,"items":{"type":"object","additionalProperties":false,"required":["id","key","name"],"properties":{
                "id":{"type":"string"},"key":{"type":"string"},"name":{"type":"string"}
            }}},"is_last":{"type":"boolean"},"start_at":integer,"max_results":integer,"total":integer,
            "next_start_at":{"type":["integer","null"],"minimum":0}
        }})),
        _ => None,
    }
}

pub(super) fn project(
    id: &str,
    payload: &Value,
    site: &url::Url,
    input: &Value,
    allowed: &[String],
) -> Result<Value, OperationError> {
    let is_last = payload["isLast"].as_bool().ok_or_else(operation_protocol)?;
    let limit = input["limit"].as_u64().ok_or_else(operation_invalid)? as usize;
    if id == "jira-issue-search" {
        let raw = payload["issues"]
            .as_array()
            .ok_or_else(operation_protocol)?;
        if raw.len() > limit {
            return Err(operation_protocol());
        }
        let mut issues = Vec::with_capacity(raw.len());
        for value in raw {
            let mut item = super::super::datasource::project_issue(
                value,
                protocol::datasource::RecordView::Detail,
                site,
            )
            .map_err(|_| operation_protocol())?
            .value;
            if issue_project(item["key"].as_str().ok_or_else(operation_protocol)?)
                != input["project_key"].as_str()
            {
                return Err(operation_protocol());
            }
            item["id"] = json!(required_string(value.get("id"), 64)?);
            issues.push(item);
        }
        let next = if is_last {
            Value::Null
        } else {
            let next = required_string(payload.get("nextPageToken"), 4096)?;
            if Some(next.as_str()) == input["next_page_token"].as_str() {
                return Err(operation_protocol());
            }
            json!(next)
        };
        Ok(json!({"issues":issues,"is_last":is_last,"next_page_token":next}))
    } else {
        let raw = payload["values"]
            .as_array()
            .ok_or_else(operation_protocol)?;
        if raw.len() > limit {
            return Err(operation_protocol());
        }
        let start = bounded_integer(payload.get("startAt"))?;
        let size = bounded_integer(payload.get("maxResults"))?;
        let total = bounded_integer(payload.get("total"))?;
        if start != input["start_at"].as_u64().unwrap_or(0)
            || (!is_last && (size == 0 || raw.is_empty()))
        {
            return Err(operation_protocol());
        }
        let mut values = Vec::new();
        for value in raw {
            let key = required_string(value.get("key"), 32)?;
            if allowed.contains(&key) {
                values.push(json!({"id":required_string(value.get("id"),64)?,"key":key,"name":required_string(value.get("name"),256)?}));
            }
        }
        Ok(
            json!({"values":values,"is_last":is_last,"start_at":start,"max_results":size,"total":total,
            "next_start_at":if is_last {None} else {Some(start.checked_add(size).ok_or_else(operation_protocol)?)} }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(id: &str, key: &str, updated: &str) -> Value {
        json!({"id":id,"key":key,"fields":{"summary":"Changed issue","status":{"name":"Open"},
            "issuetype":{"name":"Task"},"updated":updated,"description":"Issue content","customfield_123":"sentinel"},"private":"sentinel"})
    }

    #[test]
    fn overlapping_pages_keep_stable_issue_ids_revisions_and_canonical_links() {
        let site = url::Url::parse("https://example.atlassian.net").unwrap();
        let first_input =
            json!({"project_key":"PROJ","updated_since_ms":1700000000000_u64,"limit":2});
        let overlapping = issue("1", "PROJ-1", "2026-09-05T10:00:00.000+0000");
        let first = project(
            "jira-issue-search",
            &json!({"issues":[overlapping.clone()],"isLast":false,"nextPageToken":"two"}),
            &site,
            &first_input,
            &["PROJ".to_owned()],
        )
        .unwrap();
        let next_input = json!({"project_key":"PROJ","updated_since_ms":1700000000000_u64,"limit":2,"next_page_token":first["next_page_token"]});
        let last = project(
            "jira-issue-search",
            &json!({"issues":[overlapping],"isLast":true}),
            &site,
            &next_input,
            &["PROJ".to_owned()],
        )
        .unwrap();
        assert_eq!(first["issues"][0]["id"], last["issues"][0]["id"]);
        assert_eq!(first["issues"][0]["updated"], last["issues"][0]["updated"]);
        assert_eq!(
            last["issues"][0]["browser_url"],
            "https://example.atlassian.net/browse/PROJ-1"
        );
        assert!(last["is_last"].as_bool().unwrap());
        assert!(last["next_page_token"].is_null());
        assert!(!first.to_string().contains("sentinel"));
        for input in [first_input, next_input] {
            let mut url =
                url::Url::parse("https://api.atlassian.com/ex/jira/site/rest/api/2/search/jql")
                    .unwrap();
            prepare_query("jira-issue-search", &input, &mut url).unwrap();
            let query: BTreeMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(
                query["jql"],
                "project = \"PROJ\" AND updated >= 1700000000000 ORDER BY updated ASC, key ASC"
            );
            assert_eq!(query["maxResults"], "2");
            assert!(!query.contains_key("updated_since_ms"));
        }
    }

    #[test]
    fn project_filtering_does_not_hide_a_provider_continuation() {
        let site = url::Url::parse("https://example.atlassian.net").unwrap();
        let first=project("jira-project-list",&json!({"values":[{"id":"2","key":"OTHER","name":"Other"}],"isLast":false,"startAt":0,"maxResults":1,"total":2}),&site,&json!({"limit":1}),&["PROJ".to_owned()]).unwrap();
        assert_eq!(first["values"], json!([]));
        assert_eq!(first["next_start_at"], 1);
        let last=project("jira-project-list",&json!({"values":[{"id":"1","key":"PROJ","name":"Project"}],"isLast":true,"startAt":1,"maxResults":1,"total":2}),&site,&json!({"limit":1,"start_at":1}),&["PROJ".to_owned()]).unwrap();
        assert_eq!(last["values"][0]["id"], "1");
        assert!(last["next_start_at"].is_null());
    }

    #[test]
    fn missing_repeated_cursor_or_foreign_issue_refuses_the_page() {
        let site = url::Url::parse("https://example.atlassian.net").unwrap();
        let input =
            json!({"project_key":"PROJ","updated_since_ms":0,"limit":1,"next_page_token":"two"});
        for payload in [
            json!({"issues":[],"isLast":false}),
            json!({"issues":[],"isLast":false,"nextPageToken":"two"}),
            json!({"issues":[issue("1","OTHER-1","2026-09-05T10:00:00Z")],"isLast":true}),
        ] {
            assert!(project(
                "jira-issue-search",
                &payload,
                &site,
                &input,
                &["PROJ".to_owned()]
            )
            .is_err());
        }
        for id in ["jira-project-list", "jira-issue-search"] {
            assert_eq!(operation_effect(id), EffectClass::ReadOnly);
            assert_eq!(operation_approval(id), ApprovalPosture::NotRequired);
        }
    }
}
