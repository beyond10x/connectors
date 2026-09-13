// generated from gitlab_writes v1
// model digest d71a3b2285e5b421c6f596c260c21fda824356846d5876221beafc5cf7d18f9a
// contract digest 0d52075ceb7e8f55a5f1b8672be27768f0e663364671db383cfdd5fe5f27c313
// do not edit: regenerate with `ess synthesize`

//! requests — `gitlab_writes.requests`.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// MergeRequestMergeOutput — `gitlab_writes.requests.MergeRequestMergeOutput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestMergeOutput {
    /// `item` — `gitlab_writes.requests.MergeRequestMergeOutputItem`.
    pub item: MergeRequestMergeOutputItem,
    /// `provenance` — `gitlab_writes.requests.MergeRequestMergeOutputProvenance`.
    pub provenance: MergeRequestMergeOutputProvenance,
}

/// MergeRequestMergeOutputItem — `gitlab_writes.requests.MergeRequestMergeOutputItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestMergeOutputItem {
    /// `description` — `Optional<String>`.
    pub description: Option<String>,
    /// `detailed_merge_status` — `String`.
    pub detailed_merge_status: String,
    /// `draft` — `Boolean`.
    pub draft: bool,
    /// `id` — `Integer`.
    pub id: i64,
    /// `iid` — `Integer`.
    pub iid: i64,
    /// `merge_commit_sha` — `Optional<String>`.
    pub merge_commit_sha: Option<String>,
    /// `project_id` — `Integer`.
    pub project_id: i64,
    /// `sha` — `Optional<String>`.
    pub sha: Option<String>,
    /// `source_branch` — `String`.
    pub source_branch: String,
    /// `source_project_id` — `Optional<Integer>`.
    pub source_project_id: Option<i64>,
    /// `squash_commit_sha` — `Optional<String>`.
    pub squash_commit_sha: Option<String>,
    /// `state` — `String`.
    pub state: String,
    /// `target_branch` — `String`.
    pub target_branch: String,
    /// `target_project_id` — `Integer`.
    pub target_project_id: i64,
    /// `title` — `String`.
    pub title: String,
    /// `updated_at` — `String`.
    pub updated_at: String,
}

/// MergeRequestMergeOutputProvenance — `gitlab_writes.requests.MergeRequestMergeOutputProvenance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestMergeOutputProvenance {
    /// `instance` — `String`.
    pub instance: String,
    /// `observed_at_unix_ms` — `Integer`.
    pub observed_at_unix_ms: i64,
    /// `resource` — `String`.
    pub resource: String,
    /// `source_revision` — `Optional<String>`.
    pub source_revision: Option<String>,
}

/// MergeRequestMergeRequest — `gitlab_writes.requests.MergeRequestMergeRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestMergeRequest {
    /// `iid` — `Integer`.
    pub iid: i64,
    /// `pipeline_id` — `Integer`.
    pub pipeline_id: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
}

/// MergeRequestUpdateOutput — `gitlab_writes.requests.MergeRequestUpdateOutput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestUpdateOutput {
    /// `item` — `gitlab_writes.requests.MergeRequestUpdateOutputItem`.
    pub item: MergeRequestUpdateOutputItem,
    /// `provenance` — `gitlab_writes.requests.MergeRequestUpdateOutputProvenance`.
    pub provenance: MergeRequestUpdateOutputProvenance,
}

/// MergeRequestUpdateOutputItem — `gitlab_writes.requests.MergeRequestUpdateOutputItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestUpdateOutputItem {
    /// `description` — `Optional<String>`.
    pub description: Option<String>,
    /// `detailed_merge_status` — `String`.
    pub detailed_merge_status: String,
    /// `draft` — `Boolean`.
    pub draft: bool,
    /// `id` — `Integer`.
    pub id: i64,
    /// `iid` — `Integer`.
    pub iid: i64,
    /// `merge_commit_sha` — `Optional<String>`.
    pub merge_commit_sha: Option<String>,
    /// `project_id` — `Integer`.
    pub project_id: i64,
    /// `sha` — `Optional<String>`.
    pub sha: Option<String>,
    /// `source_branch` — `String`.
    pub source_branch: String,
    /// `source_project_id` — `Optional<Integer>`.
    pub source_project_id: Option<i64>,
    /// `squash_commit_sha` — `Optional<String>`.
    pub squash_commit_sha: Option<String>,
    /// `state` — `String`.
    pub state: String,
    /// `target_branch` — `String`.
    pub target_branch: String,
    /// `target_project_id` — `Integer`.
    pub target_project_id: i64,
    /// `title` — `String`.
    pub title: String,
    /// `updated_at` — `String`.
    pub updated_at: String,
}

/// MergeRequestUpdateOutputProvenance — `gitlab_writes.requests.MergeRequestUpdateOutputProvenance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestUpdateOutputProvenance {
    /// `instance` — `String`.
    pub instance: String,
    /// `observed_at_unix_ms` — `Integer`.
    pub observed_at_unix_ms: i64,
    /// `resource` — `String`.
    pub resource: String,
    /// `source_revision` — `Optional<String>`.
    pub source_revision: Option<String>,
}

/// MergeRequestUpdateRequest — `gitlab_writes.requests.MergeRequestUpdateRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestUpdateRequest {
    /// `iid` — `Integer`.
    pub iid: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
    /// `title` — `String`.
    pub title: String,
}
