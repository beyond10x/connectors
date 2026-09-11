//! Native MR observations and update-window traversal. No merge admission or
//! persistent consumer checkpoint is supplied by these reads.
use super::*;
use std::collections::BTreeSet;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn protocol() -> Error {
    Error::new(
        ErrorCode::UpstreamProtocol,
        "provider returned inconsistent MR data",
    )
}
fn positive(v: &Value) -> Result<i64> {
    v.as_i64().filter(|n| *n > 0).ok_or_else(protocol)
}
fn text(v: &Value, max: usize, empty: bool) -> Result<&str> {
    v.as_str()
        .filter(|s| (empty || !s.is_empty()) && s.len() <= max)
        .ok_or_else(protocol)
}
fn timestamp(v: &Value) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(text(v, 64, false)?, &Rfc3339).map_err(|_| protocol())
}
fn optional_sha(v: &Value) -> Result<Value> {
    if v.is_null() {
        return Ok(Value::Null);
    }
    let s = text(v, 64, false)?;
    if !matches!(s.len(), 40 | 64)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(protocol());
    }
    Ok(json!(s))
}
pub(super) fn observation(v: &Value, project: &str, iid: Option<i64>) -> Result<Value> {
    let id = positive(&v["id"])?;
    let observed_iid = positive(&v["iid"])?;
    let project_id = positive(&v["project_id"])?;
    let target = positive(&v["target_project_id"])?;
    if iid.is_some_and(|i| i != observed_iid)
        || project_id != target
        || project.parse::<i64>().is_ok_and(|i| i != project_id)
    {
        return Err(protocol());
    }
    let source = if v["source_project_id"].is_null() {
        None
    } else {
        Some(positive(&v["source_project_id"])?)
    };
    let description = if v["description"].is_null() {
        None
    } else {
        Some(text(&v["description"], 16384, true)?)
    };
    timestamp(&v["updated_at"])?;
    Ok(json!({"id":id,"iid":observed_iid,"project_id":project_id,
        "target_project_id":target,"source_project_id":source,
        "title":text(&v["title"],1024,false)?,"description":description,
        "source_branch":text(&v["source_branch"],1024,false)?,
        "target_branch":text(&v["target_branch"],1024,false)?,
        "state":text(&v["state"],64,false)?,
        "draft":v["draft"].as_bool().ok_or_else(protocol)?,
        "detailed_merge_status":text(&v["detailed_merge_status"],64,false)?,
        "sha":optional_sha(&v["sha"])?,
        "merge_commit_sha":optional_sha(&v["merge_commit_sha"])?,
        "squash_commit_sha":optional_sha(&v["squash_commit_sha"])?,
        "updated_at":v["updated_at"]}))
}
fn window(input: &MergeRequestsListRequest) -> Result<(OffsetDateTime, OffsetDateTime)> {
    let parse = |s: &str| {
        OffsetDateTime::parse(s, &Rfc3339).map_err(|_| Error::invalid("invalid MR update window"))
    };
    let after = parse(&input.updated_after)?;
    let before = parse(&input.updated_before)?;
    if after > before {
        return Err(Error::invalid("invalid MR update window"));
    }
    Ok((after, before))
}

impl GitLabBindings {
    pub(super) fn mr_finish_validation(
        &self,
        input: MergeRequestValidateRequest,
        response: HttpResponse,
    ) -> Result<Value> {
        let raw = upstream_json(&response)?;
        let mr = observation(&raw, &input.project, Some(input.iid))?;
        let pipeline = if raw["head_pipeline"].is_null() {
            Value::Null
        } else {
            let p = &raw["head_pipeline"];
            let id = positive(&p["id"])?;
            let project = positive(&p["project_id"])?;
            if (mr["source_project_id"] != project && mr["target_project_id"] != project)
                || p["sha"].is_null()
            {
                return Err(protocol());
            }
            json!({"id":id,"project_id":project,"sha":optional_sha(&p["sha"])?,
                "status":text(&p["status"],64,false)?})
        };
        let mut blockers = Vec::new();
        if mr["sha"].is_null() {
            blockers.push("head_unavailable");
        } else if mr["sha"] != input.sha {
            blockers.push("head_changed");
        }
        if mr["state"] != "opened" {
            blockers.push("not_open");
        }
        if mr["draft"] == true {
            blockers.push("draft");
        }
        if mr["detailed_merge_status"] != "mergeable" {
            blockers.push("merge_checks_pending");
        }
        if pipeline.is_null() {
            blockers.push("pipeline_unavailable");
        } else {
            if pipeline["id"] != input.pipeline_id {
                blockers.push("pipeline_changed");
            }
            if pipeline["sha"] != input.sha {
                blockers.push("pipeline_head_mismatch");
            }
            if pipeline["status"] != "success" {
                blockers.push("pipeline_not_successful");
            }
        }
        Ok(json!({"item":{"merge_request":mr,"expected_sha":input.sha,
            "expected_pipeline_id":input.pipeline_id,"head_pipeline":pipeline,
            "checks_passed":blockers.is_empty(),"blockers":blockers,"merge_performed":false},
            "provenance":provenance(&self.descriptor.instance,
                format!("{}/merge_requests/{}",input.project,input.iid),None)}))
    }
    fn mr_cursor_context(&self, input: &MergeRequestsListRequest) -> Value {
        json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,
            "partition":self.partition,"operation":"merge_requests.list",
            "project":input.project,"state":input.state,"updated_after":input.updated_after,
            "updated_before":input.updated_before,"limit":input.limit})
    }
    pub(super) fn mr_prepare_list(
        &self,
        input: &MergeRequestsListRequest,
    ) -> Result<MergeRequestsListRequestContext> {
        self.admit_project(&input.project)?;
        window(input)?;
        let page = input
            .cursor
            .as_deref()
            .map(|c| self.cursors.read(&self.mr_cursor_context(input), c))
            .transpose()?
            .unwrap_or_else(|| "1".into());
        if !page.parse::<u32>().is_ok_and(|n| n > 0) {
            return Err(Error::new(
                ErrorCode::StaleCursor,
                "invalid MR continuation",
            ));
        }
        Ok(MergeRequestsListRequestContext { page })
    }
    pub(super) fn mr_finish_get(
        &self,
        input: MergeRequestGetRequest,
        response: HttpResponse,
    ) -> Result<Value> {
        let item = observation(&upstream_json(&response)?, &input.project, Some(input.iid))?;
        Ok(
            json!({"item":item,"provenance":provenance(&self.descriptor.instance,
            format!("{}/merge_requests/{}",input.project,input.iid),None)}),
        )
    }
    pub(super) fn mr_finish_list(
        &self,
        input: MergeRequestsListRequest,
        context: MergeRequestsListRequestContext,
        response: HttpResponse,
    ) -> Result<Value> {
        let (after, before) = window(&input)?;
        let raw = upstream_json(&response)?;
        let raw = raw.as_array().ok_or_else(protocol)?;
        if raw.len() > input.limit as usize {
            return Err(protocol());
        }
        let mut ids = BTreeSet::new();
        let mut iids = BTreeSet::new();
        let mut previous = after;
        let mut items = Vec::with_capacity(raw.len());
        for raw in raw {
            let item = observation(raw, &input.project, None)?;
            let updated = timestamp(&item["updated_at"])?;
            if updated < previous
                || updated > before
                || (input.state != "all" && item["state"] != input.state)
                || !ids.insert(positive(&item["id"])?)
                || !iids.insert(positive(&item["iid"])?)
            {
                return Err(protocol());
            }
            previous = updated;
            items.push(item);
        }
        let page: u32 = context.page.parse().map_err(|_| Error::internal())?;
        let next = match response.headers.get("x-next-page") {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(
                s.parse::<u32>()
                    .ok()
                    .filter(|n| *n > page)
                    .ok_or_else(protocol)?,
            ),
            None if items.len() < input.limit as usize => None,
            None => Some(
                page.checked_add(1)
                    .ok_or_else(|| Error::new(ErrorCode::Capacity, "MR pagination limit"))?,
            ),
        };
        let next_cursor = next
            .map(|n| {
                self.cursors
                    .issue(&self.mr_cursor_context(&input), n.to_string())
            })
            .transpose()?;
        encode(Page {
            items,
            complete: next_cursor.is_none(),
            next_cursor,
            provenance: provenance(
                &self.descriptor.instance,
                format!("{}/merge_requests", input.project),
                None,
            ),
        })
    }
}
