// generated from gitlab v1
// model digest b437c9021e38b1a4dca0876effaedcbf8c5f6ce1fcb3f7212e936fd6f9356d07
// contract digest 004c3aa38f241afa38c5b5988b23bf2cc5194cd63287264f15043685f8a77bf7
// do not edit: regenerate with `ess synthesize`

//! requests — `gitlab.requests`.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// FileGetRequest — `gitlab.requests.FileGetRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGetRequest {
    /// `path` — `String`.
    pub path: String,
    /// `project` — `String`.
    pub project: String,
    /// `ref` — `String`.
    pub r#ref: String,
}

/// IssuesListRequest — `gitlab.requests.IssuesListRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuesListRequest {
    /// `cursor` — `Optional<String>`.
    pub cursor: Option<String>,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `project` — `String`.
    pub project: String,
}

/// JobGetRequest — `gitlab.requests.JobGetRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobGetRequest {
    /// `job_id` — `Integer`.
    pub job_id: i64,
    /// `pipeline_id` — `Integer`.
    pub pipeline_id: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
}

/// JobTraceRequest — `gitlab.requests.JobTraceRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobTraceRequest {
    /// `job_id` — `Integer`.
    pub job_id: i64,
    /// `max_bytes` — `Integer`.
    pub max_bytes: i64,
    /// `project` — `String`.
    pub project: String,
}

/// MergeRequestGetRequest — `gitlab.requests.MergeRequestGetRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestGetRequest {
    /// `iid` — `Integer`.
    pub iid: i64,
    /// `project` — `String`.
    pub project: String,
}

/// MergeRequestValidateRequest — `gitlab.requests.MergeRequestValidateRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestValidateRequest {
    /// `iid` — `Integer`.
    pub iid: i64,
    /// `pipeline_id` — `Integer`.
    pub pipeline_id: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
}

/// MergeRequestsListRequest — `gitlab.requests.MergeRequestsListRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequestsListRequest {
    /// `cursor` — `Optional<String>`.
    pub cursor: Option<String>,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `project` — `String`.
    pub project: String,
    /// `state` — `String`.
    pub state: String,
    /// `updated_after` — `String`.
    pub updated_after: String,
    /// `updated_before` — `String`.
    pub updated_before: String,
}

/// PipelineGetRequest — `gitlab.requests.PipelineGetRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineGetRequest {
    /// `pipeline_id` — `Integer`.
    pub pipeline_id: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
}

/// PipelineJobsRequest — `gitlab.requests.PipelineJobsRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineJobsRequest {
    /// `cursor` — `Optional<String>`.
    pub cursor: Option<String>,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `pipeline_id` — `Integer`.
    pub pipeline_id: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
}

/// PipelinesListRequest — `gitlab.requests.PipelinesListRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelinesListRequest {
    /// `cursor` — `Optional<String>`.
    pub cursor: Option<String>,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `project` — `String`.
    pub project: String,
    /// `sha` — `String`.
    pub sha: String,
}

/// ProjectGetRequest — `gitlab.requests.ProjectGetRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGetRequest {
    /// `project` — `String`.
    pub project: String,
}
