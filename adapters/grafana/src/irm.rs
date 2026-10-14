//! IRM reads (`spec/ess/domains/irm.yaml`; `design.md` beside this adapter): alert groups
//! from the IRM API URL and incidents from the stack's IRM plugin resource, each over a
//! since window the caller advances, and each incident's references to alert groups.
//! Read only: no request this module can send acknowledges, resolves, silences or writes.
//!
//! Alert groups are one `GET api/v1/alert_groups/` per page on the IRM port, which the
//! composition built with `X-Grafana-URL` fixed to the configured stack URL. Incidents are
//! one read POST per page (`IncidentsService.QueryIncidents`, `…GetIncident`) on the stack
//! port, whose two paths the composition fixed. IRM's `next` URL is never followed: the
//! cursor is the next page number, sent to the configured URL.
use crate::{Grafana, malformed};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::HttpResponse;
use serde_json::{Map, Value, json};

/// `GET api/v1/alert_groups/` (the final empty segment is the trailing `/`).
pub const ALERT_GROUPS: [&str; 4] = ["api", "v1", "alert_groups", ""];
/// The IRM plugin resource on the stack, under which the incident service answers.
const INCIDENT_SERVICE: [&str; 6] = [
    "api",
    "plugins",
    "grafana-irm-app",
    "resources",
    "api",
    "v1",
];
/// The two reads the stack port may send as a POST, and nothing else.
pub const INCIDENT_READ_POSTS: [&[&str]; 2] = [
    &[
        "api",
        "plugins",
        "grafana-irm-app",
        "resources",
        "api",
        "v1",
        "IncidentsService.QueryIncidents",
    ],
    &[
        "api",
        "plugins",
        "grafana-irm-app",
        "resources",
        "api",
        "v1",
        "IncidentsService.GetIncident",
    ],
];
/// Alert groups per page: IRM's maximum (`perpage` is "capped at 100").
pub const ALERT_GROUP_PAGE: usize = 100;
/// Incidents per page (`query.limit`); the service documents no maximum.
pub const INCIDENT_PAGE: usize = 50;
/// OnCall references read from one incident before the answer is refused.
pub const INCIDENT_REFS: usize = 64;
/// `AlertGroupRecord.title.count`, `IncidentRecord.title.count <= 4096`.
pub const TITLE_CHARS: usize = 4096;
/// `IncidentRecord.severity.count <= 64`.
pub const SEVERITY_CHARS: usize = 64;
/// `IncidentCursor.value.count <= 1024`.
pub const CURSOR_BYTES: usize = 1024;
/// The reference key under which an incident names an alert group (inferred; README).
pub const ONCALL_REF: &str = "com.grafana.oncall";

/// A provider status as the family's error, never carrying the provider's text. A refusal
/// is the provider's own answer; 404 is `not found` only for a read by id.
fn answer(response: HttpResponse, by_id: bool) -> Result<Value> {
    match response.status {
        200 => {}
        401 => {
            return Err(Error::new(ErrorCode::Unauthorized, "Grafana refused access").answered());
        }
        403 => {
            return Err(Error::new(ErrorCode::Forbidden, "Grafana refused the read").answered());
        }
        404 if by_id => {
            return Err(Error::new(ErrorCode::NotFound, "Grafana has no such record").answered());
        }
        429 => {
            return Err(
                Error::new(ErrorCode::RateLimited, "Grafana rate limit reached").answered(),
            );
        }
        _ => return Err(Error::unavailable()),
    }
    connectors_core::read_json::<Value>(&response.body).map_err(|_| malformed())
}

fn transport(error: Error) -> Error {
    match error.code {
        ErrorCode::Timeout => error,
        _ => Error::unavailable(),
    }
}

/// A window bound as the caller writes it: `YYYY-MM-DDTHH:MM:SSZ`, a real UTC instant.
/// Its text orders as its instant, so two bounds compare as strings.
fn bound(input: &Value, member: &str) -> Result<String> {
    let text = input
        .get(member)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::invalid("a window needs since and until"))?;
    if !utc_second(text) {
        return Err(Error::invalid(
            "a window bound is YYYY-MM-DDTHH:MM:SSZ in UTC",
        ));
    }
    Ok(text.to_owned())
}

fn digits(text: &str) -> Option<u32> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// The calendar part `YYYY-MM-DD` and the clock part `HH:MM:SS` are a real instant.
fn calendar(date: &str, clock: &str) -> bool {
    let date: Vec<&str> = date.split('-').collect();
    let clock: Vec<&str> = clock.split(':').collect();
    let [year, month, day] = date[..] else {
        return false;
    };
    let [hour, minute, second] = clock[..] else {
        return false;
    };
    if year.len() != 4
        || [month, day, hour, minute, second]
            .iter()
            .any(|part| part.len() != 2)
    {
        return false;
    }
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        digits(year),
        digits(month),
        digits(day),
        digits(hour),
        digits(minute),
        digits(second),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day) && hour < 24 && minute < 60 && second < 60
}

/// `YYYY-MM-DDTHH:MM:SSZ` exactly.
fn utc_second(text: &str) -> bool {
    text.len() == 20
        && text.is_ascii()
        && text.as_bytes()[10] == b'T'
        && text.ends_with('Z')
        && calendar(&text[..10], &text[11..19])
}

/// An RFC 3339 date-time as the provider answers it: whole or fractional seconds, `Z` or
/// a numeric offset. It is returned verbatim.
fn rfc3339(text: &str) -> bool {
    if !(20..=40).contains(&text.len()) || !text.is_ascii() {
        return false;
    }
    let bytes = text.as_bytes();
    if !matches!(bytes[10], b'T' | b't') || !calendar(&text[..10], &text[11..19]) {
        return false;
    }
    let mut rest = &text[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        let end = fraction
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(fraction.len());
        if end == 0 || end > 9 {
            return false;
        }
        rest = &fraction[end..];
    }
    match rest {
        "Z" | "z" => true,
        offset => {
            offset.len() == 6
                && matches!(offset.as_bytes()[0], b'+' | b'-')
                && offset.as_bytes()[3] == b':'
                && digits(&offset[1..3]).is_some_and(|h| h < 24)
                && digits(&offset[4..6]).is_some_and(|m| m < 60)
        }
    }
}

struct Window {
    since: String,
    until: String,
}

impl Window {
    fn read(input: &Value) -> Result<Self> {
        let since = bound(input, "since")?;
        let until = bound(input, "until")?;
        if since >= until {
            return Err(Error::invalid("a window's since is before its until"));
        }
        Ok(Self { since, until })
    }
    fn value(&self) -> Value {
        json!({"since": self.since, "until": self.until})
    }
    /// IRM's `started_at`: `%Y-%m-%dT%H:%M:%S_%Y-%m-%dT%H:%M:%S`, UTC.
    fn started_at(&self) -> String {
        format!("{}_{}", &self.since[..19], &self.until[..19])
    }
}

fn irm_id(value: &Value) -> Result<&str> {
    let text = value.as_str().ok_or_else(malformed)?;
    if text.is_empty() || text.len() > 64 || !text.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(malformed());
    }
    Ok(text)
}

fn incident_id(value: &Value) -> Result<&str> {
    let text = value.as_str().ok_or_else(malformed)?;
    if text.is_empty()
        || text.len() > 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(malformed());
    }
    Ok(text)
}

fn timestamp(value: &Value) -> Result<&str> {
    let text = value.as_str().ok_or_else(malformed)?;
    if !rfc3339(text) {
        return Err(malformed());
    }
    Ok(text)
}

fn bounded_text(value: &Value, chars: usize) -> Result<&str> {
    let text = value.as_str().ok_or_else(malformed)?;
    let count = text.chars().count();
    if count == 0 || count > chars {
        return Err(malformed());
    }
    Ok(text)
}

fn count(value: &Value) -> Result<u64> {
    value.as_u64().ok_or_else(malformed)
}

/// Absent and `null` are the same: the record omits the member.
fn present<'a>(source: &'a Map<String, Value>, member: &str) -> Option<&'a Value> {
    source.get(member).filter(|v| !v.is_null())
}

/// One IRM alert group as an `AlertGroupRecord`. Only the modelled members are read; a
/// missing required member or a value outside the model refuses the whole answer.
fn alert_group(source: &Value) -> Result<Value> {
    let source = source.as_object().ok_or_else(malformed)?;
    let required = |member: &str| source.get(member).ok_or_else(malformed);
    let state = match required("state")?.as_str() {
        Some(state @ ("new" | "acknowledged" | "resolved" | "silenced")) => state,
        _ => return Err(malformed()),
    };
    let mut record = Map::new();
    record.insert("id".into(), json!(irm_id(required("id")?)?));
    record.insert(
        "integration_id".into(),
        json!(irm_id(required("integration_id")?)?),
    );
    if let Some(route) = present(source, "route_id") {
        record.insert("route_id".into(), json!(irm_id(route)?));
    }
    if let Some(number) = present(source, "alert_group_number") {
        record.insert("alert_group_number".into(), json!(count(number)?));
    }
    record.insert(
        "alerts_count".into(),
        json!(count(required("alerts_count")?)?),
    );
    record.insert("state".into(), json!(state));
    if let Some(title) = present(source, "title") {
        record.insert("title".into(), json!(bounded_text(title, TITLE_CHARS)?));
    }
    record.insert(
        "created_at".into(),
        json!(timestamp(required("created_at")?)?),
    );
    for member in ["acknowledged_at", "resolved_at", "silenced_at"] {
        if let Some(at) = present(source, member) {
            record.insert(member.into(), json!(timestamp(at)?));
        }
    }
    if let Some(root) = present(source, "root_alert_group_id") {
        record.insert("root_alert_group_id".into(), json!(irm_id(root)?));
    }
    Ok(Value::Object(record))
}

/// Empty text is the service's absent value.
fn optional_text<'a>(source: &'a Map<String, Value>, member: &str) -> Result<Option<&'a Value>> {
    match source.get(member) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if text.is_empty() => Ok(None),
        Some(value @ Value::String(_)) => Ok(Some(value)),
        Some(_) => Err(malformed()),
    }
}

/// One incident as an `IncidentRecord`, and its OnCall references as links.
fn incident(source: &Value, links: &mut Vec<Value>) -> Result<Value> {
    let source = source.as_object().ok_or_else(malformed)?;
    let required = |member: &str| source.get(member).ok_or_else(malformed);
    let id = incident_id(required("incidentID")?)?;
    let status = match required("status")?.as_str() {
        Some(status @ ("active" | "resolved")) => status,
        _ => return Err(malformed()),
    };
    let mut record = Map::new();
    record.insert("incident_id".into(), json!(id));
    record.insert(
        "title".into(),
        json!(bounded_text(required("title")?, TITLE_CHARS)?),
    );
    record.insert("status".into(), json!(status));
    if let Some(severity) = optional_text(source, "severity")? {
        record.insert(
            "severity".into(),
            json!(bounded_text(severity, SEVERITY_CHARS)?),
        );
    }
    record.insert(
        "is_drill".into(),
        json!(required("isDrill")?.as_bool().ok_or_else(malformed)?),
    );
    record.insert(
        "created_time".into(),
        json!(timestamp(required("createdTime")?)?),
    );
    record.insert(
        "modified_time".into(),
        json!(timestamp(required("modifiedTime")?)?),
    );
    if let Some(closed) = optional_text(source, "closedTime")? {
        record.insert("closed_time".into(), json!(timestamp(closed)?));
    }
    let refs = match source.get("refs") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(refs)) => refs.as_slice(),
        Some(_) => return Err(malformed()),
    };
    let mut seen = Vec::new();
    for reference in refs {
        let reference = reference.as_object().ok_or_else(malformed)?;
        if reference.get("key").and_then(Value::as_str) != Some(ONCALL_REF) {
            continue;
        }
        let group = irm_id(reference.get("ref").ok_or_else(malformed)?)?;
        if !seen.contains(&group) {
            if seen.len() == INCIDENT_REFS {
                return Err(malformed());
            }
            seen.push(group);
        }
    }
    links.extend(
        seen.into_iter()
            .map(|group| json!({"incident_id": id, "alert_group_id": group})),
    );
    Ok(Value::Object(record))
}

impl Grafana {
    fn provenance(&self, resource: &str) -> Value {
        json!({
            "instance": self.instance,
            "resource": resource,
            "observed_at_unix_ms": (self.clock)(),
            "source_revision": null,
        })
    }

    fn irm_port(&self) -> Result<&dyn connectors_sdk::AuthenticatedHttp> {
        // Without `irm_api_url` no IRM read is composed; nothing is sent.
        self.irm.as_deref().ok_or_else(Error::unavailable)
    }

    pub(crate) async fn alert_groups(&self, input: &Value) -> Result<Value> {
        let irm = self.irm_port()?;
        let window = Window::read(input)?;
        let page: u32 = match input.get("cursor") {
            None => 1,
            Some(cursor) => cursor
                .as_str()
                .filter(|c| !c.starts_with('0'))
                .and_then(digits)
                .filter(|p| (1..=999_999).contains(p))
                .ok_or_else(|| Error::invalid("not an alert-group cursor"))?,
        };
        let mut query = vec![
            ("started_at", window.started_at()),
            ("perpage", ALERT_GROUP_PAGE.to_string()),
            ("page", page.to_string()),
        ];
        for member in ["state", "integration_id", "team_id"] {
            if let Some(value) = input.get(member).and_then(Value::as_str) {
                query.push((member, value.to_owned()));
            }
        }
        let response = irm.get(&ALERT_GROUPS, &query).await.map_err(transport)?;
        let answer = answer(response, false)?;
        let results = answer
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(malformed)?;
        if results.len() > ALERT_GROUP_PAGE {
            return Err(malformed());
        }
        let items = results
            .iter()
            .map(alert_group)
            .collect::<Result<Vec<_>>>()?;
        let more = match answer.get("next") {
            Some(Value::Null) => false,
            Some(Value::String(_)) => true,
            _ => return Err(malformed()),
        };
        if more && page == 999_999 {
            return Err(malformed());
        }
        Ok(json!({
            "items": items,
            "complete": !more,
            "next_cursor": more.then(|| (page + 1).to_string()),
            "window": window.value(),
            "provenance": self.provenance("grafana:irm.alert_groups"),
        }))
    }

    pub(crate) async fn alert_group(&self, input: &Value) -> Result<Value> {
        let irm = self.irm_port()?;
        let id = input
            .get("alert_group_id")
            .and_then(Value::as_str)
            .filter(|id| irm_id(&json!(id)).is_ok())
            .ok_or_else(|| Error::invalid("not an alert-group id"))?;
        let response = irm
            .get(&["api", "v1", "alert_groups", id], &[])
            .await
            .map_err(transport)?;
        let item = alert_group(&answer(response, true)?)?;
        Ok(json!({
            "item": item,
            "provenance": self.provenance("grafana:irm.alert_group"),
        }))
    }

    async fn incident_service(&self, method: &str, body: &Value, by_id: bool) -> Result<Value> {
        let mut path = INCIDENT_SERVICE.to_vec();
        path.push(method);
        let response = self
            .http
            .post_json(&path, &[], body)
            .await
            .map_err(transport)?;
        let answer = answer(response, by_id)?;
        // A legacy server may report a failure in the body of a 2xx answer.
        match answer.get("error") {
            None | Some(Value::Null) => {}
            Some(Value::String(error)) if error.is_empty() => {}
            Some(_) => return Err(Error::unavailable()),
        }
        Ok(answer)
    }

    pub(crate) async fn incidents(&self, input: &Value) -> Result<Value> {
        self.irm_port()?;
        let window = Window::read(input)?;
        let cursor = match input.get("cursor") {
            None => None,
            Some(cursor) => Some(
                cursor
                    .as_str()
                    .filter(|c| cursor_text(c))
                    .ok_or_else(|| Error::invalid("not an incident cursor"))?,
            ),
        };
        // `queryString` is never sent: with it the service ignores the window.
        let body = json!({
            "query": {
                "limit": INCIDENT_PAGE,
                "dateFrom": window.since,
                "dateTo": window.until,
                "orderDirection": "ASC",
            },
            "cursor": {
                "nextValue": cursor.unwrap_or(""),
                "hasMore": cursor.is_some(),
            },
        });
        let answer = self
            .incident_service("IncidentsService.QueryIncidents", &body, false)
            .await?;
        let listed = answer
            .get("incidents")
            .and_then(Value::as_array)
            .ok_or_else(malformed)?;
        if listed.len() > INCIDENT_PAGE {
            return Err(malformed());
        }
        let mut links = Vec::new();
        let items = listed
            .iter()
            .map(|i| incident(i, &mut links))
            .collect::<Result<Vec<_>>>()?;
        let cursor = answer
            .get("cursor")
            .and_then(Value::as_object)
            .ok_or_else(malformed)?;
        let more = cursor
            .get("hasMore")
            .and_then(Value::as_bool)
            .ok_or_else(malformed)?;
        let next = if more {
            let next = cursor
                .get("nextValue")
                .and_then(Value::as_str)
                .filter(|c| cursor_text(c))
                .ok_or_else(malformed)?;
            Some(next.to_owned())
        } else {
            None
        };
        Ok(json!({
            "items": items,
            "links": links,
            "complete": !more,
            "next_cursor": next,
            "window": window.value(),
            "provenance": self.provenance("grafana:irm.incidents"),
        }))
    }

    pub(crate) async fn incident(&self, input: &Value) -> Result<Value> {
        self.irm_port()?;
        let id = input
            .get("incident_id")
            .and_then(Value::as_str)
            .filter(|id| incident_id(&json!(id)).is_ok())
            .ok_or_else(|| Error::invalid("not an incident id"))?;
        let answer = self
            .incident_service(
                "IncidentsService.GetIncident",
                &json!({ "incidentID": id }),
                true,
            )
            .await?;
        let mut links = Vec::new();
        let item = incident(answer.get("incident").ok_or_else(malformed)?, &mut links)?;
        Ok(json!({
            "item": item,
            "links": links,
            "provenance": self.provenance("grafana:irm.incident"),
        }))
    }
}

/// An incident cursor: 1–1,024 visible ASCII characters.
fn cursor_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= CURSOR_BYTES && text.bytes().all(|b| b.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_timestamps_are_rfc3339_date_times() {
        for admitted in [
            "2026-01-01T12:37:01Z",
            "2026-01-01T12:37:01.430444Z",
            "2026-01-01T12:37:01.123456789+05:30",
            "2028-02-29T00:00:00-00:00",
        ] {
            assert!(rfc3339(admitted), "{admitted}");
        }
        for refused in [
            "",
            "2026-01-01",
            "2026-01-01 12:37:01Z",
            "2026-01-01T12:37:01",
            "2026-01-01T12:37:01.Z",
            "2026-01-01T12:37:01.1234567890Z",
            "2026-02-29T00:00:00Z",
            "2026-01-01T12:37:01+5:30",
            "2026-01-01T12:37:01+24:00",
            "２026-01-01T12:37:01Z",
        ] {
            assert!(!rfc3339(refused), "{refused}");
        }
    }
}
