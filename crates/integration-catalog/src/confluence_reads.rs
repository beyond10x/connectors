//! Safe projection of the added incremental search, preserving provider continuation.
use protocol::operation::{OperationError, OperationErrorCode};
use serde_json::{json, Map, Value};

pub(crate) fn validate_input(input: &Value) -> Result<(), OperationError> {
    let invalid = || {
        OperationError::new(
            OperationErrorCode::InvalidInput,
            "Confluence page search requires bounded cql, limit, cursor and the declared expansion",
            false,
        )
    };
    let object = input.as_object().ok_or_else(invalid)?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "cql" | "limit" | "cursor" | "expand"))
        || !input["cql"]
            .as_str()
            .is_some_and(|text| !text.trim().is_empty() && text.len() <= 4096)
        || !input["limit"]
            .as_u64()
            .is_some_and(|limit| (1..=100).contains(&limit))
        || input["expand"] != "version,body.storage,space"
        || object.get("cursor").is_some_and(|value| {
            !value
                .as_str()
                .is_some_and(|text| !text.is_empty() && text.len() <= 4096)
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "Confluence page search response violates its bounded projection",
        false,
    )
}
fn pick(value: &Value, names: &[&str]) -> Result<Value, OperationError> {
    let source = value.as_object().ok_or_else(invalid)?;
    let mut projected = Map::new();
    for name in names {
        let Some(value) = source.get(*name) else {
            continue;
        };
        let valid = match *name {
            "id" => value.as_u64().is_some() || value.as_str().is_some_and(|s| !s.is_empty()),
            "number" => value.as_u64().is_some_and(|n| n > 0),
            "size" | "limit" => value.as_u64().is_some_and(|n| n <= 100),
            "next" => value.is_null() || value.is_string(),
            "representation" => value == "storage",
            _ => value.is_string(),
        };
        if !valid {
            return Err(invalid());
        }
        projected.insert((*name).to_owned(), value.clone());
    }
    Ok(Value::Object(projected))
}
pub(crate) fn project(value: Value) -> Result<Value, OperationError> {
    let results = value["results"].as_array().ok_or_else(invalid)?;
    if results.len() > 100 {
        return Err(invalid());
    }
    let mut pages = Vec::with_capacity(results.len());
    for source in results {
        if source["type"] != "page"
            || !source["id"].is_string()
            || !source["title"].is_string()
            || source["version"]["number"].as_u64().is_none()
            || !source["version"]["when"].is_string()
            || !source["body"]["storage"]["value"].is_string()
        {
            return Err(invalid());
        }
        if !source["space"]["key"].is_string() || !source["_links"]["webui"].is_string() {
            return Err(invalid());
        }
        let mut page = pick(source, &["id", "title", "type", "status"])?;
        page["version"] = pick(&source["version"], &["number", "when"])?;
        page["body"] =
            json!({"storage":pick(&source["body"]["storage"],&["value","representation"])?});
        page["space"] = pick(&source["space"], &["id", "key", "name"])?;
        page["_links"] = pick(&source["_links"], &["webui", "base"])?;
        pages.push(page);
    }
    let mut projected = pick(&value, &["size", "limit"])?;
    projected["results"] = Value::Array(pages);
    projected["_links"] = pick(&value["_links"], &["next", "base"])?;
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_inputs_remain_closed_bounded_and_expansion_pinned() {
        let valid = json!({"cql":"type=page AND space=DOCS","limit":50,"expand":"version,body.storage,space"});
        assert!(validate_input(&valid).is_ok());
        for (field, value) in [
            ("cql", json!(" ")),
            ("cql", json!("x".repeat(4097))),
            ("limit", json!(0)),
            ("limit", json!(101)),
            ("limit", json!("50")),
            ("expand", json!("history.contributors")),
            ("cursor", json!({})),
            ("cursor", json!("")),
            ("cursor", Value::Null),
            ("extra", json!(true)),
        ] {
            let mut input = valid.clone();
            input[field] = value;
            assert!(validate_input(&input).is_err(), "{field}: {input}");
        }
    }
    #[test]
    fn overlapping_pages_keep_versions_content_and_continuation_without_person_records() {
        let page = json!({"id":"1","title":"Page","type":"page","status":"current",
            "version":{"number":2,"when":"2026-09-05T10:00:00Z","by":{"email":"private@example.test"}},
            "body":{"storage":{"value":"<p>Content</p>","representation":"storage"}},
            "space":{"id":7,"key":"DOCS","name":"Documentation","private":"sentinel"},
            "_links":{"webui":"/pages/1"},"history":{"author":"private"}});
        let first=project(json!({"results":[page.clone()],"size":1,"limit":2,"_links":{"next":"/rest/api/content/search?cursor=two","base":"https://example.test/wiki"}})).unwrap();
        let last=project(json!({"results":[page],"size":1,"limit":2,"_links":{"base":"https://example.test/wiki"}})).unwrap();
        assert_eq!(first["results"], last["results"]);
        assert!(first["_links"]["next"].is_string());
        assert!(last["_links"]["next"].is_null());
        assert!(!first.to_string().contains("private"));
        assert!(!first.to_string().contains("sentinel"));
        assert_eq!(first["results"][0]["version"]["number"], 2);
    }
    #[test]
    fn missing_revision_is_not_a_successful_observation() {
        assert!(project(json!({"results":[{"id":"1"}],"_links":{}})).is_err());
    }
    #[test]
    fn a_declared_link_cannot_smuggle_an_untyped_object() {
        assert!(project(json!({"results":[],"size":0,"limit":2,
            "_links":{"next":{"unexpected":"private"},"base":"https://example.test/wiki"}}))
        .is_err());
    }
}
