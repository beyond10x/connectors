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
    Adapter, AuthenticatedHttp, Cursors, HttpResponse, encode, instance_descriptor, provenance,
    upstream_json,
};
use gitlab_types::requests::{FileGetRequest, IssuesListRequest, ProjectGetRequest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub mod auth;

#[path = "../generated/runtime.rs"]
#[doc(hidden)]
#[rustfmt::skip]
pub mod generated;
use generated::{
    Bindings, FileGetRequestContext, IssuesListRequestContext, ProjectGetRequestContext,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub allowed_projects: Vec<String>,
}

pub struct GitLab(generated::GeneratedAdapter<GitLabBindings>);
struct GitLabBindings {
    config: Config,
    descriptor: Descriptor,
    cursors: Cursors,
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
        Ok(Self(generated::GeneratedAdapter {
            http,
            descriptor: descriptor.clone(),
            bindings: GitLabBindings {
                config,
                descriptor,
                cursors: Cursors::default(),
            },
        }))
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
        json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,
            "operation":"issues.list","project":input.project,"limit":input.limit})
    }
}

impl Bindings for GitLabBindings {
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
