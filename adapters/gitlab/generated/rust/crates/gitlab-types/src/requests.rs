// generated from gitlab v1
// model digest ede8787ae4303e8248e5bd6de3675b12866d725713b94852f37db003d4982392
// contract digest 9020464eeb141c4af3f7cd1c704975f1a25aa17da6a44f4b61ff3a5c25400334
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
