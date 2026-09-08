// generated from gitlab v1
// model digest 5464a959b36fe5673bd570eecf3f661540dd261b60c0fde2fd37f4a5bf2d97f3
// contract digest c3d3a9b408c90c2a8b03daf08984379733603f2c84f483640772f01ea8aea190
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

/// ProjectGetRequest — `gitlab.requests.ProjectGetRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGetRequest {
    /// `project` — `String`.
    pub project: String,
}
