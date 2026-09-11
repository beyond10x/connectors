//! Fresh native preflight and one consuming SHA-guarded merge. Host authority,
//! proof custody, attempt storage and approval spend are outside this library.
use super::*;
use connectors_sdk::{AuthenticatedWrite, WriteOutcome};
use gitlab_writes_types::requests::{
    MergeRequestMergeOutput, MergeRequestMergeOutputItem, MergeRequestMergeOutputProvenance,
    MergeRequestMergeRequest,
};

pub struct PreparedMerge(writes::PreparedWrite<MergeBindings>);
impl PreparedMerge {
    pub async fn execute(self, http: Box<dyn AuthenticatedWrite>) -> WriteOutcome<Value> {
        self.0.execute(http).await
    }
}

struct MergeBindings {
    instance: String,
    input: Value,
    project_id: i64,
    mr_id: i64,
}
impl GitLab {
    /// Reads the current MR through the same generated validation path as the
    /// public observation. The returned preparation retains no read capability.
    pub async fn prepare_merge(
        &self,
        http: Arc<dyn AuthenticatedHttp>,
        partition: &str,
        input: Value,
    ) -> Result<PreparedMerge> {
        let current = self
            .with_authenticated_http(http, partition)?
            .invoke("merge_request.validate", input.clone())
            .await?;
        if current["item"]["checks_passed"] != true {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "current merge checks refused",
            ));
        }
        let mr = &current["item"]["merge_request"];
        let bindings = MergeBindings {
            instance: self.0.descriptor.instance.clone(),
            input: input.clone(),
            project_id: mr["project_id"].as_i64().ok_or_else(Error::internal)?,
            mr_id: mr["id"].as_i64().ok_or_else(Error::internal)?,
        };
        writes::prepare(
            &self.0.bindings.private_descriptor,
            Arc::new(bindings),
            "merge_request.merge",
            input,
        )
        .map(PreparedMerge)
    }
}
impl writes::WriteBindings for MergeBindings {
    fn prepare_merge_request_merge(
        &self,
        input: &MergeRequestMergeRequest,
    ) -> Result<writes::MergeRequestMergeRequestContext> {
        if self.input
            != json!({"project":input.project,"iid":input.iid,"sha":input.sha,"pipeline_id":input.pipeline_id})
        {
            return Err(Error::internal());
        }
        Ok(writes::MergeRequestMergeRequestContext {})
    }
    fn finish_merge_request_merge(
        &self,
        input: MergeRequestMergeRequest,
        _: writes::MergeRequestMergeRequestContext,
        response: Result<HttpResponse>,
    ) -> WriteOutcome<MergeRequestMergeOutput> {
        let response = match response {
            Ok(response) => response,
            Err(_) => return WriteOutcome::Unknown(Error::unavailable()),
        };
        // Only documented definite native refusals are classified as such.
        // Neither retry hints nor arbitrary response text is returned or acted on.
        if matches!(response.status, 403 | 405 | 409 | 422) {
            return WriteOutcome::Refused(Error::new(ErrorCode::Forbidden, "native merge refused"));
        }
        let protocol = || {
            Error::new(
                ErrorCode::UpstreamProtocol,
                "unconfirmed native merge outcome",
            )
        };
        if response.status != 200 {
            return WriteOutcome::Unknown(protocol());
        }
        let item = match upstream_json(&response)
            .and_then(|raw| merge_requests::observation(&raw, &input.project, Some(input.iid)))
        {
            Ok(item) => item,
            Err(_) => return WriteOutcome::Unknown(protocol()),
        };
        // Numeric identity captured in preflight also binds path-based selectors.
        if item["project_id"] != self.project_id
            || item["id"] != self.mr_id
            || item["sha"] != input.sha
            || item["state"] != "merged"
        {
            return WriteOutcome::Unknown(protocol());
        }
        // From this point the exact target is known merged, even if projection fails.
        WriteOutcome::Applied((|| {
            let text = |key: &str| item[key].as_str().map(str::to_owned).ok_or_else(protocol);
            let integer = |key: &str| item[key].as_i64().ok_or_else(protocol);
            Ok(MergeRequestMergeOutput {
                item: MergeRequestMergeOutputItem {
                    description: item["description"].as_str().map(str::to_owned),
                    detailed_merge_status: text("detailed_merge_status")?,
                    draft: item["draft"].as_bool().ok_or_else(protocol)?,
                    id: integer("id")?,
                    iid: integer("iid")?,
                    merge_commit_sha: item["merge_commit_sha"].as_str().map(str::to_owned),
                    project_id: integer("project_id")?,
                    sha: Some(input.sha),
                    source_branch: text("source_branch")?,
                    source_project_id: item["source_project_id"].as_i64(),
                    squash_commit_sha: item["squash_commit_sha"].as_str().map(str::to_owned),
                    state: text("state")?,
                    target_branch: text("target_branch")?,
                    target_project_id: integer("target_project_id")?,
                    title: text("title")?,
                    updated_at: text("updated_at")?,
                },
                provenance: MergeRequestMergeOutputProvenance {
                    instance: self.instance.clone(),
                    resource: format!("{}/merge_requests/{}", input.project, input.iid),
                    observed_at_unix_ms: connectors_sdk::now_ms()
                        .try_into()
                        .map_err(|_| protocol())?,
                    source_revision: None,
                },
            })
        })())
    }
}
