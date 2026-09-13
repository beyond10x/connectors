//! Fresh native preflight, one unguarded title update, then a postflight
//! comparison. GitLab's merge-request update carries no source-SHA precondition,
//! so the window between the preflight read and the mutation cannot be closed.
//! A moved head makes the outcome **possibly applied**, never refused, and this
//! library never issues a corrective mutation to hide it.
use super::*;
use connectors_sdk::{AuthenticatedWrite, WriteOutcome};
use gitlab_writes_types::requests::{
    MergeRequestUpdateOutput, MergeRequestUpdateOutputItem, MergeRequestUpdateOutputProvenance,
    MergeRequestUpdateRequest,
};

pub struct PreparedUpdate(writes::PreparedWrite<UpdateBindings>);
impl PreparedUpdate {
    pub async fn execute(self, http: Box<dyn AuthenticatedWrite>) -> WriteOutcome<Value> {
        self.0.execute(http).await
    }
}

pub struct UpdateBindings {
    instance: String,
    input: Value,
    project_id: i64,
    mr_id: i64,
    expected_sha: String,
}
impl GitLab {
    /// Reads the current merge request through the same generated path the public
    /// observation uses, and refuses before any write when the head already
    /// differs from the pinned SHA. The returned preparation retains no read
    /// capability.
    pub async fn prepare_update(
        &self,
        http: Arc<dyn AuthenticatedHttp>,
        partition: &str,
        input: Value,
    ) -> Result<PreparedUpdate> {
        let selector = json!({"project": input["project"], "iid": input["iid"]});
        let current = self
            .with_authenticated_http(http, partition)?
            .invoke("merge_request.get", selector)
            .await?;
        let mr = &current["item"];
        let expected = input["sha"].as_str().ok_or_else(Error::internal)?;
        if mr["state"] != "opened" {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "merge request is not open",
            ));
        }
        // A preflight refusal is the only place a head difference is definite.
        // After dispatch the same difference is uncertainty, not a refusal.
        if mr["sha"].as_str() != Some(expected) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "merge request head differs from the pinned source SHA",
            ));
        }
        let bindings = UpdateBindings {
            instance: self.0.descriptor.instance.clone(),
            input: input.clone(),
            project_id: mr["project_id"].as_i64().ok_or_else(Error::internal)?,
            mr_id: mr["id"].as_i64().ok_or_else(Error::internal)?,
            expected_sha: expected.to_owned(),
        };
        writes::prepare(
            &self.0.bindings.private_descriptor,
            Arc::new(bindings),
            "merge_request.update",
            input,
        )
        .map(PreparedUpdate)
    }
}
impl writes::WriteBindings for UpdateBindings {
    fn prepare_merge_request_merge(
        &self,
        _: &gitlab_writes_types::requests::MergeRequestMergeRequest,
    ) -> Result<writes::MergeRequestMergeRequestContext> {
        Err(Error::internal())
    }
    fn finish_merge_request_merge(
        &self,
        _: gitlab_writes_types::requests::MergeRequestMergeRequest,
        _: writes::MergeRequestMergeRequestContext,
        _: Result<HttpResponse>,
    ) -> WriteOutcome<gitlab_writes_types::requests::MergeRequestMergeOutput> {
        WriteOutcome::Unknown(Error::internal())
    }
    fn prepare_merge_request_update(
        &self,
        input: &MergeRequestUpdateRequest,
    ) -> Result<writes::MergeRequestUpdateRequestContext> {
        if self.input
            != json!({"project":input.project,"iid":input.iid,"title":input.title,"sha":input.sha})
        {
            return Err(Error::internal());
        }
        Ok(writes::MergeRequestUpdateRequestContext {})
    }
    fn finish_merge_request_update(
        &self,
        input: MergeRequestUpdateRequest,
        _: writes::MergeRequestUpdateRequestContext,
        response: Result<HttpResponse>,
    ) -> WriteOutcome<MergeRequestUpdateOutput> {
        let response = match response {
            Ok(response) => response,
            Err(_) => return WriteOutcome::Unknown(Error::unavailable()),
        };
        // Only documented definite native refusals are classified as such.
        if matches!(response.status, 403 | 404 | 405 | 409 | 422) {
            return WriteOutcome::Refused(Error::new(
                ErrorCode::Forbidden,
                "native merge request update refused",
            ));
        }
        let protocol = || {
            Error::new(
                ErrorCode::UpstreamProtocol,
                "unconfirmed native merge request update outcome",
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
        if item["project_id"] != self.project_id || item["id"] != self.mr_id {
            return WriteOutcome::Unknown(protocol());
        }
        // The accepted C14 boundary. GitLab offers no source-SHA precondition on
        // update, so a head that moved between preflight and dispatch leaves the
        // effect possible rather than refused. Reporting it as refused would be a
        // false negative about a write the provider may already have applied.
        //
        // This comparison is best effort and not detection. A merge request's
        // recorded head is eventually consistent with its source branch: on
        // 2026-09-13 a live GitLab answered this PUT with the pinned head while the
        // branch had already moved, and the move was visible on the very next read.
        // A match here means the move was not observed, never that none happened.
        if item["sha"].as_str() != Some(self.expected_sha.as_str()) {
            return WriteOutcome::Unknown(Error::new(
                ErrorCode::UpstreamProtocol,
                "merge request head moved during the update; the effect is possible",
            ));
        }
        if item["title"] != input.title {
            return WriteOutcome::Unknown(protocol());
        }
        // From this point the exact target is known updated, even if projection fails.
        WriteOutcome::Applied((|| {
            let text = |key: &str| item[key].as_str().map(str::to_owned).ok_or_else(protocol);
            let integer = |key: &str| item[key].as_i64().ok_or_else(protocol);
            Ok(MergeRequestUpdateOutput {
                item: MergeRequestUpdateOutputItem {
                    description: item["description"].as_str().map(str::to_owned),
                    detailed_merge_status: text("detailed_merge_status")?,
                    draft: item["draft"].as_bool().ok_or_else(protocol)?,
                    id: integer("id")?,
                    iid: integer("iid")?,
                    merge_commit_sha: item["merge_commit_sha"].as_str().map(str::to_owned),
                    project_id: integer("project_id")?,
                    sha: Some(self.expected_sha.clone()),
                    source_branch: text("source_branch")?,
                    source_project_id: item["source_project_id"].as_i64(),
                    squash_commit_sha: item["squash_commit_sha"].as_str().map(str::to_owned),
                    state: text("state")?,
                    target_branch: text("target_branch")?,
                    target_project_id: integer("target_project_id")?,
                    title: text("title")?,
                    updated_at: text("updated_at")?,
                },
                provenance: MergeRequestUpdateOutputProvenance {
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
