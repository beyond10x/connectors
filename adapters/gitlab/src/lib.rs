//! The generated runtime requires every handwritten obligation at compile time.
//!
//! ```compile_fail,E0046
//! struct MissingImplementations;
//! impl connectors_gitlab::generated::Bindings for MissingImplementations {}
//! ```
use async_trait::async_trait;
use connectors_contracts::Page;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{
    Adapter, AuthenticatedHttp, Cursors, HttpResponse, HttpResponsePrefix, encode,
    instance_descriptor, provenance, upstream_json,
};
use gitlab_types::requests::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub mod auth;
mod ci;
mod merge;
mod merge_requests;
pub use merge::PreparedMerge;

#[path = "../generated/writes.rs"]
#[rustfmt::skip]
mod writes;

#[path = "../generated/runtime.rs"]
#[doc(hidden)]
#[rustfmt::skip]
pub mod generated;
use generated::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub allowed_projects: Vec<String>,
}

pub struct GitLab(generated::GeneratedAdapter<GitLabBindings>);
struct GitLabBindings {
    config: Config,
    descriptor: Descriptor,
    private_descriptor: Descriptor,
    cursors: Arc<Cursors>,
    partition: Option<String>,
}
impl GitLab {
    pub fn new(
        instance: &str,
        config: Config,
        effective_configuration: Value,
        http: Arc<dyn AuthenticatedHttp>,
    ) -> Result<Self> {
        if config.allowed_projects.is_empty()
            || config
                .allowed_projects
                .iter()
                .any(|p| p.is_empty() || p.len() > 512)
        {
            return Err(Error::invalid("configure a nonempty project allowlist"));
        }
        let descriptor = instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            &effective_configuration,
        )?;
        generated::verify_descriptor(&descriptor)?;
        let private_descriptor = instance_descriptor(
            include_str!("../generated/private-descriptor.json"),
            instance,
            &effective_configuration,
        )?;
        Ok(Self(generated::GeneratedAdapter {
            http,
            descriptor: descriptor.clone(),
            bindings: GitLabBindings {
                config,
                descriptor,
                private_descriptor,
                cursors: Arc::new(Cursors::default()),
                partition: None,
            },
        }))
    }
    /// A new immutable HTTP capability for one host-admitted use. Cursor state
    /// stays adapter-owned and is partitioned by an opaque authenticated binding.
    /// This value grants no credential lookup, publication or dispatch authority.
    pub fn with_authenticated_http(
        &self,
        http: Arc<dyn AuthenticatedHttp>,
        partition: &str,
    ) -> Result<Self> {
        if !connectors_core::valid_id(partition) {
            return Err(Error::invalid("invalid cursor partition"));
        }
        Ok(Self(generated::GeneratedAdapter {
            http,
            descriptor: self.0.descriptor.clone(),
            bindings: GitLabBindings {
                config: self.0.bindings.config.clone(),
                descriptor: self.0.bindings.descriptor.clone(),
                private_descriptor: self.0.bindings.private_descriptor.clone(),
                cursors: self.0.bindings.cursors.clone(),
                partition: Some(partition.to_owned()),
            },
        }))
    }
    /// Full local projection, selected only by private protocol two. The public
    /// Adapter implementation continues to expose its read-only descriptor.
    pub fn private_descriptor(&self) -> Descriptor {
        self.0.bindings.private_descriptor.clone()
    }
}
impl GitLabBindings {
    fn admit_project(&self, project: &str) -> Result<()> {
        if !self.config.allowed_projects.iter().any(|p| p == project) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "project is outside the configured scope",
            ));
        }
        Ok(())
    }
    fn cursor_context(&self, input: &IssuesListRequest) -> Value {
        let mut context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,
            "operation":"issues.list","project":input.project,"limit":input.limit});
        if let Some(partition) = &self.partition {
            context["partition"] = json!(partition);
        }
        context
    }
}

impl Bindings for GitLabBindings {
    fn prepare_merge_request_validate(
        &self,
        input: &MergeRequestValidateRequest,
    ) -> Result<MergeRequestValidateRequestContext> {
        self.admit_project(&input.project)?;
        Ok(MergeRequestValidateRequestContext {})
    }
    fn finish_merge_request_validate(
        &self,
        input: MergeRequestValidateRequest,
        _: MergeRequestValidateRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        self.mr_finish_validation(input, response)
    }
    fn prepare_merge_request_get(
        &self,
        input: &MergeRequestGetRequest,
    ) -> Result<MergeRequestGetRequestContext> {
        self.admit_project(&input.project)?;
        Ok(MergeRequestGetRequestContext {})
    }
    fn finish_merge_request_get(
        &self,
        input: MergeRequestGetRequest,
        _: MergeRequestGetRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        self.mr_finish_get(input, response)
    }
    fn prepare_merge_requests_list(
        &self,
        input: &MergeRequestsListRequest,
    ) -> Result<MergeRequestsListRequestContext> {
        self.mr_prepare_list(input)
    }
    fn finish_merge_requests_list(
        &self,
        input: MergeRequestsListRequest,
        context: MergeRequestsListRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        self.mr_finish_list(input, context, response)
    }
    fn prepare_pipelines_list(
        &self,
        input: &PipelinesListRequest,
    ) -> Result<PipelinesListRequestContext> {
        self.admit_project(&input.project)?;
        Ok(PipelinesListRequestContext {
            page: self.ci_page_number(
                &self.ci_cursor_context(
                    "pipelines.list",
                    &input.project,
                    &input.sha,
                    None,
                    input.limit,
                ),
                input.cursor.as_deref(),
            )?,
        })
    }
    fn finish_pipelines_list(
        &self,
        input: PipelinesListRequest,
        context: PipelinesListRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let cursor = self.ci_cursor_context(
            "pipelines.list",
            &input.project,
            &input.sha,
            None,
            input.limit,
        );
        self.ci_page(response, &context.page, &cursor, &input.project, |v| {
            ci::pipeline(v, &input.project, &input.sha, None)
        })
    }
    fn prepare_pipeline_get(
        &self,
        input: &PipelineGetRequest,
    ) -> Result<PipelineGetRequestContext> {
        self.admit_project(&input.project)?;
        Ok(PipelineGetRequestContext {})
    }
    fn finish_pipeline_get(
        &self,
        input: PipelineGetRequest,
        _: PipelineGetRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let item = ci::pipeline(
            &upstream_json(&response)?,
            &input.project,
            &input.sha,
            Some(input.pipeline_id),
        )?;
        Ok(
            json!({"item":item,"provenance":provenance(&self.descriptor.instance,
            format!("{}/pipelines/{}",input.project,input.pipeline_id),Some(input.sha))}),
        )
    }
    fn prepare_pipeline_jobs(
        &self,
        input: &PipelineJobsRequest,
    ) -> Result<PipelineJobsRequestContext> {
        self.admit_project(&input.project)?;
        Ok(PipelineJobsRequestContext {
            page: self.ci_page_number(
                &self.ci_cursor_context(
                    "pipeline.jobs",
                    &input.project,
                    &input.sha,
                    Some(input.pipeline_id),
                    input.limit,
                ),
                input.cursor.as_deref(),
            )?,
        })
    }
    fn finish_pipeline_jobs(
        &self,
        input: PipelineJobsRequest,
        context: PipelineJobsRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let cursor = self.ci_cursor_context(
            "pipeline.jobs",
            &input.project,
            &input.sha,
            Some(input.pipeline_id),
            input.limit,
        );
        self.ci_page(
            response,
            &context.page,
            &cursor,
            &format!("{}/pipelines/{}/jobs", input.project, input.pipeline_id),
            |v| ci::job(v, &input.sha, input.pipeline_id, None),
        )
    }
    fn prepare_job_get(&self, input: &JobGetRequest) -> Result<JobGetRequestContext> {
        self.admit_project(&input.project)?;
        Ok(JobGetRequestContext {})
    }
    fn finish_job_get(
        &self,
        input: JobGetRequest,
        _: JobGetRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let item = ci::job(
            &upstream_json(&response)?,
            &input.sha,
            input.pipeline_id,
            Some(input.job_id),
        )?;
        Ok(
            json!({"item":item,"provenance":provenance(&self.descriptor.instance,
            format!("{}/jobs/{}",input.project,input.job_id),Some(input.sha))}),
        )
    }
    fn prepare_job_trace(&self, input: &JobTraceRequest) -> Result<JobTraceRequestContext> {
        self.admit_project(&input.project)?;
        Ok(JobTraceRequestContext {})
    }
    fn finish_job_trace(
        &self,
        input: JobTraceRequest,
        _: JobTraceRequestContext,
        response: HttpResponsePrefix,
    ) -> Result<Value> {
        let item = ci::trace(response, input.job_id, input.max_bytes)?;
        Ok(
            json!({"item":item,"provenance":provenance(&self.descriptor.instance,
            format!("{}/jobs/{}/trace",input.project,input.job_id),None)}),
        )
    }
    fn prepare_project_get(&self, input: &ProjectGetRequest) -> Result<ProjectGetRequestContext> {
        self.admit_project(&input.project)?;
        Ok(ProjectGetRequestContext {})
    }
    fn finish_project_get(
        &self,
        input: ProjectGetRequest,
        _: ProjectGetRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let item = upstream_json(&response)?;
        if !item.is_object() || !item["id"].is_number() {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "provider returned an invalid project",
            ));
        }
        Ok(
            json!({"item":item,"provenance":provenance(&self.descriptor.instance,input.project,None)}),
        )
    }
    fn prepare_issues_list(&self, input: &IssuesListRequest) -> Result<IssuesListRequestContext> {
        self.admit_project(&input.project)?;
        let page = match &input.cursor {
            Some(cursor) => self.cursors.read(&self.cursor_context(input), cursor)?,
            None => "1".into(),
        };
        let number: u32 = page
            .parse()
            .map_err(|_| Error::new(ErrorCode::StaleCursor, "invalid provider continuation"))?;
        if number == 0 {
            return Err(Error::new(
                ErrorCode::StaleCursor,
                "invalid provider continuation",
            ));
        }
        Ok(IssuesListRequestContext { page })
    }
    fn finish_issues_list(
        &self,
        input: IssuesListRequest,
        context: IssuesListRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let page_number: u32 = context.page.parse().map_err(|_| Error::internal())?;
        let value = upstream_json(&response)?;
        let items = value
            .as_array()
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UpstreamProtocol,
                    "provider did not return an issue list",
                )
            })?
            .clone();
        if items.len() > input.limit as usize {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "provider exceeded requested page size",
            ));
        }
        let next = match response.headers.get("x-next-page") {
            Some(value) if value.is_empty() => None,
            Some(value) => {
                let number: u32 = value.parse().map_err(|_| {
                    Error::new(ErrorCode::UpstreamProtocol, "invalid pagination header")
                })?;
                if number <= page_number {
                    return Err(Error::new(
                        ErrorCode::UpstreamProtocol,
                        "nonadvancing pagination",
                    ));
                }
                Some(number.to_string())
            }
            None if items.len() < input.limit as usize => None,
            None => Some(
                page_number
                    .checked_add(1)
                    .ok_or_else(|| Error::new(ErrorCode::Capacity, "pagination limit"))?
                    .to_string(),
            ),
        };
        let next_cursor = next
            .map(|page| self.cursors.issue(&self.cursor_context(&input), page))
            .transpose()?;
        encode(Page {
            items,
            complete: next_cursor.is_none(),
            next_cursor,
            provenance: provenance(&self.descriptor.instance, input.project, None),
        })
    }
    fn prepare_file_get(&self, input: &FileGetRequest) -> Result<FileGetRequestContext> {
        self.admit_project(&input.project)?;
        Ok(FileGetRequestContext {})
    }
    fn finish_file_get(
        &self,
        input: FileGetRequest,
        _: FileGetRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let item = upstream_json(&response)?;
        if item["encoding"] != "base64" || !item["content"].is_string() {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "provider returned an unsupported file representation",
            ));
        }
        let revision = item["commit_id"].as_str().map(str::to_owned);
        Ok(
            json!({"item":item,"provenance":provenance(&self.descriptor.instance,format!("{}/{}",input.project,input.path),revision)}),
        )
    }
}

#[async_trait]
impl Adapter for GitLab {
    fn descriptor(&self) -> Descriptor {
        self.0.descriptor()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        self.0.invoke(operation, input).await
    }
}
