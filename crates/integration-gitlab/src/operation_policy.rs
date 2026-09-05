//! GitLab operation identity and read/write admission policy.
use super::*;

pub(super) const GITLAB_OPERATIONS: [&str; 19] = [
    "gitlab-project-activity-list",
    "gitlab-pipeline-list",
    "gitlab-deployment-list",
    "gitlab-repository-commit-list",
    "gitlab-user-get",
    "gitlab-group-list",
    "gitlab-project-list",
    "gitlab-issue-list",
    "gitlab-issue-get",
    "gitlab-issue-create",
    "gitlab-merge-request-list",
    "gitlab-merge-request-create",
    "gitlab-merge-request-update",
    "gitlab-pipeline-get",
    "gitlab-branch-list",
    "gitlab-branch-create",
    "gitlab-repository-commit-create",
    "gitlab-repository-tree-list",
    REPOSITORY_FILE_GET,
];

pub(super) fn supports_operation(connection: &StoredConnection, operation_ref: &str) -> bool {
    is_gitlab_operation(operation_ref)
        && (!is_mutating_operation(operation_ref)
            || connection.scopes.iter().any(|scope| scope == "api"))
}

pub(super) fn is_gitlab_operation(value: &str) -> bool {
    GITLAB_OPERATIONS.contains(&value)
}

pub(super) fn operation_effect(operation_ref: &str) -> EffectClass {
    if is_mutating_operation(operation_ref) {
        EffectClass::Mutating
    } else {
        EffectClass::ReadOnly
    }
}

pub(super) fn operation_approval(operation_ref: &str) -> ApprovalPosture {
    if is_mutating_operation(operation_ref) {
        ApprovalPosture::Required
    } else {
        ApprovalPosture::NotRequired
    }
}

pub(super) fn is_mutating_operation(operation_ref: &str) -> bool {
    matches!(
        operation_ref,
        "gitlab-issue-create"
            | "gitlab-merge-request-create"
            | "gitlab-merge-request-update"
            | "gitlab-branch-create"
            | "gitlab-repository-commit-create"
    )
}
