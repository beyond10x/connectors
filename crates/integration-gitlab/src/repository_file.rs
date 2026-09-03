//! The one GitLab operation whose input names a nested repository path.
//!
//! `GET /projects/:id/repository/files/:file_path` takes the whole repository-relative path as one
//! URL-encoded segment. The generic resolver refuses a `/` inside a path parameter, which is right
//! for every other operation, so the path is validated here first and placed onto the resolved URL
//! as a single segment afterwards; the permission subject is the URL that results.
//!
//! Moved out of `backend.rs` unchanged on 2026-09-04. `d3707aa` added both functions there and
//! took the file past its size waiver, which is what failed the release gate for v0.5.3 through
//! v0.5.6; the waiver admits no further growth before the backend's arms split, and this was the
//! arm that grew.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::backend::REPOSITORY_FILE_GET;

pub(crate) fn resolve_operation_plan(
    operation: &connector_resolve::document::Operation,
    base: &str,
    input: &Value,
    credentials: &[connector_resolve::auth::Assembled],
) -> Result<connector_resolve::RequestPlan, ()> {
    let mut resolved_input = input.clone();
    let repository_file_path = if operation.id == REPOSITORY_FILE_GET {
        let path = input
            .get("file_path")
            .and_then(Value::as_str)
            .filter(|path| valid_repository_file_path(path))
            .ok_or(())?
            .to_owned();
        resolved_input
            .as_object_mut()
            .ok_or(())?
            .insert("file_path".to_owned(), Value::String("file".to_owned()));
        Some(path)
    } else {
        None
    };
    let mut plan = connector_resolve::resolve(
        operation,
        base,
        &resolved_input,
        &BTreeMap::new(),
        credentials,
    )
    .map_err(|_| ())?;
    if let Some(path) = repository_file_path {
        let mut target = url::Url::parse(&plan.request.url).map_err(|_| ())?;
        target
            .path_segments_mut()
            .map_err(|_| ())?
            .pop()
            .push(&path);
        plan.request.url = target.to_string();
        plan.permission_subjects = vec![plan.request.url.clone()];
    }
    Ok(plan)
}

pub(crate) fn valid_repository_file_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 3_072
        && !path.starts_with('/')
        && !path.contains(['\0', '\\'])
        && path
            .split('/')
            .all(|component| !component.is_empty() && !matches!(component, "." | ".." | ".git"))
}
