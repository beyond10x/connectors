use async_trait::async_trait;
use connectors_contracts::Page;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{
    Adapter, AuthenticatedHttp, Cursors, decode, encode, instance_descriptor, provenance,
    upstream_json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub allowed_projects: Vec<String>,
}

pub struct GitLab {
    http: Arc<dyn AuthenticatedHttp>,
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
        connectors_sdk::verify_handlers(&descriptor, &["project.get", "issues.list", "file.get"])?;
        Ok(Self {
            http,
            config,
            descriptor,
            cursors: Cursors::default(),
        })
    }
    fn admit_project(&self, project: &str) -> Result<()> {
        if !self.config.allowed_projects.iter().any(|p| p == project) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "project is outside the configured scope",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    project: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Issues {
    project: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    project: String,
    path: String,
    #[serde(rename = "ref")]
    reference: String,
}

#[async_trait]
impl Adapter for GitLab {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        match operation {
            "project.get" => {
                let args: Project = decode(input)?;
                self.admit_project(&args.project)?;
                let response = self.http.get(&["projects", &args.project], &[]).await?;
                let item = upstream_json(&response)?;
                if !item.is_object() || !item["id"].is_number() {
                    return Err(Error::new(
                        ErrorCode::UpstreamProtocol,
                        "provider returned an invalid project",
                    ));
                }
                Ok(
                    json!({"item":item,"provenance":provenance(&self.descriptor.instance,args.project,None)}),
                )
            }
            "issues.list" => {
                let args: Issues = decode(input)?;
                self.admit_project(&args.project)?;
                if !(1..=100).contains(&args.limit) {
                    return Err(Error::invalid("limit must be between one and 100"));
                }
                let context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,"operation":operation,"project":args.project,"limit":args.limit});
                let page = match args.cursor {
                    Some(ref c) => self.cursors.read(&context, c)?,
                    None => "1".into(),
                };
                let page_number: u32 = page.parse().map_err(|_| {
                    Error::new(ErrorCode::StaleCursor, "invalid provider continuation")
                })?;
                let response = self
                    .http
                    .get(
                        &["projects", &args.project, "issues"],
                        &[
                            ("per_page", args.limit.to_string()),
                            ("page", page),
                            ("order_by", "created_at".into()),
                            ("sort", "asc".into()),
                        ],
                    )
                    .await?;
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
                if items.len() > args.limit as usize {
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
                    None if items.len() < args.limit as usize => None,
                    None => Some(
                        page_number
                            .checked_add(1)
                            .ok_or_else(|| Error::new(ErrorCode::Capacity, "pagination limit"))?
                            .to_string(),
                    ),
                };
                let next_cursor = next.map(|v| self.cursors.issue(&context, v)).transpose()?;
                encode(Page {
                    items,
                    complete: next_cursor.is_none(),
                    next_cursor,
                    provenance: provenance(&self.descriptor.instance, args.project, None),
                })
            }
            "file.get" => {
                let args: File = decode(input)?;
                self.admit_project(&args.project)?;
                if args.path.is_empty()
                    || args.path.len() > 4096
                    || args.reference.is_empty()
                    || args.reference.len() > 1024
                {
                    return Err(Error::invalid("invalid file path or revision"));
                }
                let response = self
                    .http
                    .get(
                        &["projects", &args.project, "repository", "files", &args.path],
                        &[("ref", args.reference)],
                    )
                    .await?;
                let item = upstream_json(&response)?;
                if item["encoding"] != "base64" || !item["content"].is_string() {
                    return Err(Error::new(
                        ErrorCode::UpstreamProtocol,
                        "provider returned an unsupported file representation",
                    ));
                }
                let revision = item["commit_id"].as_str().map(str::to_owned);
                Ok(
                    json!({"item":item,"provenance":provenance(&self.descriptor.instance,format!("{}/{}",args.project,args.path),revision)}),
                )
            }
            _ => Err(Error::new(
                ErrorCode::NotFound,
                "operation is not implemented",
            )),
        }
    }
}
