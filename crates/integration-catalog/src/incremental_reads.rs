//! Placement-local incremental adaptation above the existing catalog credential and egress seams.
//! Legacy operation contracts are untouched. These reads have explicit bounded contracts.
use protocol::operation::{OperationError, OperationErrorCode};
use serde_json::{json, Value};
use service::EgressHttpResponse;

mod comments;
mod gitlab;
mod jira;

pub(super) fn handles(id: &str) -> bool {
    matches!(
        id,
        "jira-issue-search"
            | "jira-project-list"
            | "jira-issue-comments-read"
            | "confluence-page-search"
            | "gitlab-issue-search"
            | "gitlab-merge-request-search"
            | "gitlab-project-activity-list"
            | "gitlab-pipeline-list"
            | "gitlab-deployment-list"
            | "gitlab-repository-commit-list"
    )
}

pub(super) fn input_schema(id: &str) -> Option<Value> {
    if id.starts_with("jira-") {
        jira::input_schema(id)
    } else if id.starts_with("gitlab-") {
        gitlab::input_schema(id)
    } else if id == "confluence-page-search" {
        Some(json!({"type":"object","additionalProperties":false,
        "required":["cql","limit","expand"],"properties":{
            "cql":{"type":"string","minLength":1,"maxLength":4096},
            "cursor":{"type":"string","minLength":1,"maxLength":4096},
            "limit":{"type":"integer","minimum":1,"maximum":100},
            "expand":{"const":"version,body.storage,space"}
        }}))
    } else {
        None
    }
}

pub(super) fn validate_input(id: &str, input: &Value) -> Result<(), OperationError> {
    if !handles(id) {
        return Ok(());
    }
    let schema = input_schema(id).ok_or_else(invalid)?;
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|_| invalid())?;
    if !validator.is_valid(input) {
        return Err(invalid());
    }
    if input.as_object().is_some_and(|object| {
        object.values().any(|value| {
            value
                .as_str()
                .is_some_and(|value| value.trim().is_empty() || value.chars().any(char::is_control))
        })
    }) {
        return Err(invalid());
    }
    if id == "confluence-page-search" {
        super::confluence_reads::validate_input(input)?;
    }
    Ok(())
}

pub(super) fn prepare_request(
    id: &str,
    input: &Value,
    request: &mut connector_resolve::Request,
) -> Result<(), OperationError> {
    if id == "jira-issue-search" {
        jira::prepare_request(input, request)?;
    } else if id == "gitlab-deployment-list"
        && (input.get("updated_after").is_some() || input.get("updated_before").is_some())
    {
        // GitLab requires update-date filtering and updated_at ordering together.
        let mut target = url::Url::parse(&request.url).map_err(|_| invalid())?;
        target
            .query_pairs_mut()
            .append_pair("order_by", "updated_at");
        request.url = target.into();
    }
    Ok(())
}

pub(super) fn response_headers(id: &str) -> Vec<String> {
    if handles(id) && id.starts_with("gitlab-") {
        vec!["x-next-page".to_owned(), "link".to_owned()]
    } else {
        Vec::new()
    }
}

pub(super) fn project(
    id: &str,
    input: &Value,
    request_url: &str,
    response: EgressHttpResponse,
) -> Result<Value, OperationError> {
    if !response.is_success() {
        let (code, retriable) = match response.status {
            401 | 403 | 404 => (OperationErrorCode::NotGranted, false),
            429 | 500..=599 => (OperationErrorCode::Unavailable, true),
            _ => (OperationErrorCode::InvalidInput, false),
        };
        return Err(OperationError::new(
            code,
            format!("Incremental read refused with HTTP {}", response.status),
            retriable,
        ));
    }
    let payload: Value = serde_json::from_slice(&response.body).map_err(|_| protocol())?;
    if id.starts_with("jira-") {
        jira::project(id, input, request_url, &payload)
    } else if id.starts_with("gitlab-") {
        gitlab::project(id, input, &response, &payload)
    } else {
        let output = super::confluence_reads::project(payload)?;
        if output["results"]
            .as_array()
            .is_none_or(|values| values.len() > input["limit"].as_u64().unwrap_or(0) as usize)
        {
            return Err(protocol());
        }
        Ok(output)
    }
}

fn invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "Incremental read input violates its closed bounded contract",
        false,
    )
}
fn protocol() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "Incremental read response violates its projection or continuation contract",
        false,
    )
}
fn string(value: &Value, maximum: usize) -> Result<String, OperationError> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= maximum)
        .map(str::to_owned)
        .ok_or_else(protocol)
}
