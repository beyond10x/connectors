//! Native CI projection and continuation rules. Transport and credential custody
//! remain injected capabilities; this module never accesses local metadata.
use super::*;

fn protocol() -> Error {
    Error::new(
        ErrorCode::UpstreamProtocol,
        "provider returned inconsistent CI data",
    )
}
fn positive(value: &Value) -> Result<i64> {
    value.as_i64().filter(|v| *v > 0).ok_or_else(protocol)
}
fn text(value: &Value, max: usize) -> Result<&str> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max)
        .ok_or_else(protocol)
}
pub(super) fn pipeline(
    value: &Value,
    project: &str,
    sha: &str,
    expected_id: Option<i64>,
) -> Result<Value> {
    let id = positive(&value["id"])?;
    let project_id = positive(&value["project_id"])?;
    if value["sha"] != sha
        || expected_id.is_some_and(|v| id != v)
        || project.parse::<i64>().is_ok_and(|v| project_id != v)
    {
        return Err(protocol());
    }
    Ok(json!({"id":id,"project_id":project_id,"sha":sha,
        "ref":text(&value["ref"],1024)?,"status":text(&value["status"],64)?}))
}
pub(super) fn job(
    value: &Value,
    sha: &str,
    pipeline_id: i64,
    expected_id: Option<i64>,
) -> Result<Value> {
    let id = positive(&value["id"])?;
    if expected_id.is_some_and(|v| v != id)
        || positive(&value["pipeline"]["id"])? != pipeline_id
        || value["pipeline"]["sha"] != sha
    {
        return Err(protocol());
    }
    Ok(json!({"id":id,"pipeline_id":pipeline_id,"sha":sha,
        "name":text(&value["name"],1024)?,"stage":text(&value["stage"],1024)?,
        "status":text(&value["status"],64)?,
        "allow_failure":value["allow_failure"].as_bool().ok_or_else(protocol)?}))
}
pub(super) fn trace(response: HttpResponsePrefix, job_id: i64, max_bytes: i64) -> Result<Value> {
    if response.status != 200 {
        // Status interpretation is shared; discard raw trace/error bytes before
        // invoking that codec, so they cannot be confused with response JSON.
        upstream_json(&HttpResponse {
            status: response.status,
            headers: response.headers,
            body: Vec::new(),
        })?;
        return Err(protocol());
    }
    if response.body.len() > 512_000 {
        return Err(protocol());
    }
    let text = match std::str::from_utf8(&response.body) {
        Ok(text) => text,
        Err(error) if error.error_len().is_none() && !response.complete => {
            std::str::from_utf8(&response.body[..error.valid_up_to()]).map_err(|_| protocol())?
        }
        Err(_) => return Err(protocol()),
    };
    let mut end = text.len().min(max_bytes as usize);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Ok(json!({"job_id":job_id,"content":&text[..end],"bytes":end,
        "complete":response.complete && end == response.body.len()}))
}

impl GitLabBindings {
    pub(super) fn ci_cursor_context(
        &self,
        operation: &str,
        project: &str,
        sha: &str,
        pipeline_id: Option<i64>,
        limit: i64,
    ) -> Value {
        json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,
            "partition":self.partition,"operation":operation,"project":project,
            "sha":sha,"pipeline_id":pipeline_id,"limit":limit})
    }
    pub(super) fn ci_page_number(&self, context: &Value, cursor: Option<&str>) -> Result<String> {
        let page = cursor
            .map(|c| self.cursors.read(context, c))
            .transpose()?
            .unwrap_or_else(|| "1".into());
        if page.parse::<u32>().is_err() || page.parse::<u32>() == Ok(0) {
            return Err(Error::new(
                ErrorCode::StaleCursor,
                "invalid CI continuation",
            ));
        }
        Ok(page)
    }
    pub(super) fn ci_page(
        &self,
        response: HttpResponse,
        page: &str,
        context: &Value,
        resource: &str,
        project: impl Fn(&Value) -> Result<Value>,
    ) -> Result<Value> {
        let limit = context["limit"].as_u64().ok_or_else(Error::internal)? as usize;
        let sha = context["sha"].as_str().ok_or_else(Error::internal)?;
        let value = upstream_json(&response)?;
        let raw = value.as_array().ok_or_else(protocol)?;
        if raw.len() > limit {
            return Err(protocol());
        }
        let items = raw.iter().map(project).collect::<Result<Vec<_>>>()?;
        // Both selected native lists promise descending IDs. Do not silently
        // reorder duplicates or a provider response that violates its selection.
        if items
            .windows(2)
            .any(|pair| pair[0]["id"].as_i64() <= pair[1]["id"].as_i64())
        {
            return Err(protocol());
        }
        let page: u32 = page.parse().map_err(|_| Error::internal())?;
        let next = match response.headers.get("x-next-page") {
            Some(value) if value.is_empty() => None,
            Some(value) => Some(
                value
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n > page)
                    .ok_or_else(protocol)?,
            ),
            None if items.len() < limit => None,
            None => Some(
                page.checked_add(1)
                    .ok_or_else(|| Error::new(ErrorCode::Capacity, "CI pagination limit"))?,
            ),
        };
        let next_cursor = next
            .map(|n| self.cursors.issue(context, n.to_string()))
            .transpose()?;
        encode(Page {
            items,
            complete: next_cursor.is_none(),
            next_cursor,
            provenance: provenance(&self.descriptor.instance, resource, Some(sha.to_owned())),
        })
    }
}
