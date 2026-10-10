//! Prometheus binding of the profiles `promql-instant`, `promql-range` and
//! `prometheus-rules` (`contracts/series/v1alpha1/semantics.md` beside this
//! adapter, §§3–5 and 11). Every operation is one GET through
//! `AuthenticatedHttp`; nothing is retained between calls (§11.3).
pub mod auth;

use async_trait::async_trait;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde::Deserialize;
use serde_json::{Map, Number, Value, json};
use std::sync::Arc;

const OPERATIONS: [&str; 3] = ["series.query", "series.query_range", "rules.list"];
const QUERY: [&str; 3] = ["api", "v1", "query"];
const QUERY_RANGE: [&str; 3] = ["api", "v1", "query_range"];
const RULES: [&str; 3] = ["api", "v1", "rules"];

/// The widest range window (§4): seven days.
pub const WINDOW_S: u64 = 604_800;
/// Series and per-series sample bounds (§5), the defaults of the optional inputs.
pub const SERIES: usize = 500;
pub const SAMPLES: usize = 2000;
/// Rules returned by one `rules.list` (§11.2).
pub const RULE_LIMIT: usize = 2000;
/// Warnings or infos returned, each clipped to `NOTE_BYTES` (§11).
pub const NOTES: usize = 32;
pub const NOTE_BYTES: usize = 256;
/// Label bounds (§11, as the logs family's §5).
pub const LABELS: usize = 128;
pub const VALUE_BYTES: usize = 4096;
pub const LABEL_BYTES: usize = 32 * 1024;
/// The widest admitted timestamp in whole seconds.
pub const MAX_UNIX_S: u64 = 9_999_999_999;

pub struct Prometheus {
    instance: String,
    descriptor: Descriptor,
    http: Arc<dyn AuthenticatedHttp>,
    clock: fn() -> u64,
}

impl Prometheus {
    /// `effective_configuration` must admit the explicit empty
    /// `query_scope.allowed_matchers`; no PromQL parser exists to enforce a
    /// nonempty one (§11.3), so the descriptor's configuration schema refuses it.
    pub fn new(
        instance: &str,
        effective_configuration: &Value,
        http: Arc<dyn AuthenticatedHttp>,
    ) -> Result<Self> {
        let descriptor = connectors_sdk::instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            effective_configuration,
        )?;
        connectors_sdk::verify_handlers(&descriptor, &OPERATIONS)?;
        Ok(Self {
            instance: instance.to_owned(),
            descriptor,
            http,
            clock: connectors_sdk::now_ms,
        })
    }

    /// A new immutable HTTP capability for one host-admitted use.
    pub fn with_authenticated_http(&self, http: Arc<dyn AuthenticatedHttp>) -> Self {
        Self {
            instance: self.instance.clone(),
            descriptor: self.descriptor.clone(),
            http,
            clock: self.clock,
        }
    }

    /// For tests: a fixed clock in milliseconds since the Unix epoch.
    pub fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    fn input_schema(&self, operation: &str) -> Result<&Value> {
        self.descriptor
            .operations
            .iter()
            .find(|o| o.id == operation)
            .map(|o| &o.input_schema)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "operation is not provided"))
    }

    fn provenance(&self, resource: &str) -> Value {
        json!({
            "instance": self.instance,
            "resource": resource,
            "observed_at_unix_ms": (self.clock)(),
            "source_revision": null,
        })
    }

    /// One GET; the validated success envelope.
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<Answer> {
        let response = self.http.get(path, query).await.map_err(|e| match e.code {
            ErrorCode::Timeout => e,
            _ => Error::unavailable(),
        })?;
        envelope(response)
    }
}

/// A success envelope: its `data`, and its bounded notes.
struct Answer {
    data: Value,
    warnings: Vec<String>,
    infos: Vec<String>,
    /// A note was omitted past `NOTES`.
    notes_omitted: bool,
}

/// A provider status as the family's error, never carrying the provider's text.
fn envelope(response: HttpResponse) -> Result<Answer> {
    match response.status {
        200 => {}
        400 => return Err(Error::invalid("Prometheus refused the query")),
        401 => {
            return Err(Error::new(
                ErrorCode::Unauthorized,
                "Prometheus refused access",
            ));
        }
        403 => {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Prometheus refused the read",
            ));
        }
        422 => {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "Prometheus could not execute the query",
            ));
        }
        429 => {
            return Err(Error::new(
                ErrorCode::RateLimited,
                "Prometheus rate limit reached",
            ));
        }
        _ => return Err(Error::unavailable()),
    }
    let Ok(Value::Object(mut body)) = connectors_core::read_json::<Value>(&response.body) else {
        return Err(malformed());
    };
    if body
        .keys()
        .any(|k| !matches!(k.as_str(), "status" | "data" | "warnings" | "infos"))
        || body.get("status").and_then(Value::as_str) != Some("success")
    {
        return Err(malformed());
    }
    let data = body.remove("data").ok_or_else(malformed)?;
    let (warnings, warnings_omitted) = notes(body.remove("warnings"))?;
    let (infos, infos_omitted) = notes(body.remove("infos"))?;
    Ok(Answer {
        data,
        warnings,
        infos,
        notes_omitted: warnings_omitted || infos_omitted,
    })
}

fn malformed() -> Error {
    Error::new(ErrorCode::Unavailable, "malformed Prometheus answer")
}

fn decode<T: for<'de> Deserialize<'de>>(input: Value) -> Result<T> {
    serde_json::from_value(input).map_err(|_| Error::invalid("input does not match the operation"))
}

/// The longest UTF-8 prefix within `limit` bytes.
fn clip(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// An optional array of strings: at most `NOTES` kept, each clipped.
fn notes(value: Option<Value>) -> Result<(Vec<String>, bool)> {
    let items = match value {
        None => return Ok((Vec::new(), false)),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(malformed()),
    };
    let mut kept = Vec::with_capacity(items.len().min(NOTES));
    for item in &items {
        let text = item.as_str().ok_or_else(malformed)?;
        if kept.len() < NOTES {
            kept.push(clip(text, NOTE_BYTES).to_owned());
        }
    }
    Ok((kept, items.len() > NOTES))
}

/// A label set within the bounds, as a JSON object of strings. Absent or null
/// is empty where `optional`.
fn labels(value: Option<&Value>, optional: bool) -> Result<Value> {
    let map = match value {
        Some(Value::Object(map)) => map,
        None | Some(Value::Null) if optional => return Ok(json!({})),
        _ => return Err(malformed()),
    };
    if map.len() > LABELS {
        return Err(malformed());
    }
    let mut size = 2;
    for (name, value) in map {
        let value = value.as_str().ok_or_else(malformed)?;
        if name.len() > 128 || value.len() > VALUE_BYTES {
            return Err(malformed());
        }
        size += name.len() + value.len() + 6;
    }
    if size > LABEL_BYTES {
        return Err(malformed());
    }
    Ok(Value::Object(map.clone()))
}

/// One `[timestamp, "value"]` pair, kept verbatim: the timestamp as the
/// provider's number, the value as its string. `within` bounds the timestamp.
fn sample(pair: &Value, within: Option<(u64, u64)>) -> Result<(f64, Value)> {
    let Some([Value::Number(timestamp), Value::String(value)]) = pair.as_array().map(Vec::as_slice)
    else {
        return Err(malformed());
    };
    let seconds = timestamp
        .as_f64()
        .filter(|s| s.is_finite() && *s >= 0.0)
        .ok_or_else(malformed)?;
    if let Some((start, end)) = within
        && (seconds < start as f64 || seconds > end as f64)
    {
        return Err(malformed());
    }
    Ok((
        seconds,
        Value::Array(vec![
            Value::Number(timestamp.clone()),
            Value::from(value.as_str()),
        ]),
    ))
}

/// The `data` object of a query answer: its result type and result.
fn query_data(data: Value) -> Result<(String, Value)> {
    let Value::Object(mut data) = data else {
        return Err(malformed());
    };
    if data
        .keys()
        .any(|k| !matches!(k.as_str(), "resultType" | "result"))
    {
        return Err(malformed());
    }
    let kind = match data.remove("resultType") {
        Some(Value::String(kind)) => kind,
        _ => return Err(malformed()),
    };
    let result = data.remove("result").ok_or_else(malformed)?;
    Ok((kind, result))
}

/// Bounded series of one validated result.
struct Series {
    series: Vec<Value>,
    series_truncated: bool,
    samples_truncated: bool,
}

/// Validate the whole result, then keep at most `max_series` series of at most
/// `max_samples` samples each (§11 "Series").
fn series(
    kind: &str,
    result: &Value,
    max_series: usize,
    max_samples: usize,
    within: Option<(u64, u64)>,
) -> Result<Series> {
    let raw: Vec<(Value, Vec<(f64, Value)>)> = match kind {
        "scalar" | "string" => vec![(json!({}), vec![sample(result, within)?])],
        "vector" | "matrix" => {
            let items = result.as_array().ok_or_else(malformed)?;
            let mut raw = Vec::with_capacity(items.len());
            for item in items {
                let object = item.as_object().ok_or_else(malformed)?;
                if object.len() != 2 {
                    return Err(malformed());
                }
                let metric = labels(object.get("metric"), false)?;
                let samples = if kind == "vector" {
                    vec![sample(object.get("value").ok_or_else(malformed)?, within)?]
                } else {
                    let values = object
                        .get("values")
                        .and_then(Value::as_array)
                        .ok_or_else(malformed)?;
                    let samples = values
                        .iter()
                        .map(|p| sample(p, within))
                        .collect::<Result<Vec<_>>>()?;
                    if samples.windows(2).any(|w| w[0].0 >= w[1].0) {
                        return Err(malformed());
                    }
                    samples
                };
                raw.push((metric, samples));
            }
            raw
        }
        _ => return Err(malformed()),
    };
    let series_truncated = raw.len() > max_series;
    let mut samples_truncated = false;
    let series = raw
        .into_iter()
        .take(max_series)
        .map(|(labels, samples)| {
            let truncated = samples.len() > max_samples;
            samples_truncated |= truncated;
            let samples: Vec<Value> = samples
                .into_iter()
                .take(max_samples)
                .map(|(_, pair)| pair)
                .collect();
            json!({"labels": labels, "samples": samples, "samples_truncated": truncated})
        })
        .collect();
    Ok(Series {
        series,
        series_truncated,
        samples_truncated,
    })
}

/// Decimal seconds as whole milliseconds (§11.2).
fn milliseconds(value: &Value) -> Result<u64> {
    let seconds = value
        .as_f64()
        .filter(|s| s.is_finite() && *s >= 0.0 && *s <= MAX_UNIX_S as f64)
        .ok_or_else(malformed)?;
    Ok((seconds * 1000.0).round() as u64)
}

fn text<'a>(object: &'a Map<String, Value>, key: &str, limit: usize) -> Result<&'a str> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= limit)
        .ok_or_else(malformed)
}

fn bounds(max_series: Option<usize>, max_samples: Option<usize>) -> Result<(usize, usize)> {
    let max_series = max_series.unwrap_or(SERIES);
    let max_samples = max_samples.unwrap_or(SAMPLES);
    if !(1..=SERIES).contains(&max_series) || !(1..=SAMPLES).contains(&max_samples) {
        return Err(Error::invalid(
            "max_series or max_samples_per_series out of range",
        ));
    }
    Ok((max_series, max_samples))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Instant {
    query: String,
    time_unix_s: Option<u64>,
    max_series: Option<usize>,
    max_samples_per_series: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    query: String,
    start_unix_s: u64,
    end_unix_s: u64,
    step_s: u64,
    max_series: Option<usize>,
    max_samples_per_series: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rules {
    kind: Option<String>,
}

impl Prometheus {
    async fn query(&self, input: Instant) -> Result<Value> {
        let (max_series, max_samples) = bounds(input.max_series, input.max_samples_per_series)?;
        let time = input.time_unix_s.unwrap_or((self.clock)() / 1000);
        if time > MAX_UNIX_S {
            return Err(Error::invalid("invalid evaluation time"));
        }
        let answer = self
            .get(
                &QUERY,
                &[("query", input.query), ("time", time.to_string())],
            )
            .await?;
        let (kind, result) = query_data(answer.data)?;
        let out = series(&kind, &result, max_series, max_samples, None)?;
        let complete = answer.warnings.is_empty()
            && !answer.notes_omitted
            && !out.series_truncated
            && !out.samples_truncated;
        Ok(json!({
            "result_type": kind,
            "series": out.series,
            "series_truncated": out.series_truncated,
            "complete": complete,
            "warnings": answer.warnings,
            "infos": answer.infos,
            "time_unix_s": time,
            "provenance": self.provenance("prometheus:query"),
        }))
    }

    async fn query_range(&self, input: Range) -> Result<Value> {
        let (max_series, max_samples) = bounds(input.max_series, input.max_samples_per_series)?;
        let (start, end, step) = (input.start_unix_s, input.end_unix_s, input.step_s);
        if end > MAX_UNIX_S || start >= end || end - start > WINDOW_S {
            return Err(Error::invalid(
                "the window must have start before end and span at most seven days",
            ));
        }
        if !(1..=WINDOW_S).contains(&step) || (end - start) / step + 1 > max_samples as u64 {
            return Err(Error::invalid(
                "the step must give at most max_samples_per_series points",
            ));
        }
        let answer = self
            .get(
                &QUERY_RANGE,
                &[
                    ("query", input.query),
                    ("start", start.to_string()),
                    ("end", end.to_string()),
                    ("step", step.to_string()),
                ],
            )
            .await?;
        let (kind, result) = query_data(answer.data)?;
        if kind != "matrix" {
            return Err(malformed());
        }
        let out = series(&kind, &result, max_series, max_samples, Some((start, end)))?;
        let complete = answer.warnings.is_empty()
            && !answer.notes_omitted
            && !out.series_truncated
            && !out.samples_truncated;
        Ok(json!({
            "result_type": "matrix",
            "series": out.series,
            "series_truncated": out.series_truncated,
            "complete": complete,
            "warnings": answer.warnings,
            "infos": answer.infos,
            "window": {"start_unix_s": start, "end_unix_s": end, "step_s": step},
            "provenance": self.provenance("prometheus:query_range"),
        }))
    }

    async fn rules(&self, input: Rules) -> Result<Value> {
        let query = match input.kind.as_deref() {
            None => vec![],
            Some("alerting") => vec![("type", "alert".to_owned())],
            Some("recording") => vec![("type", "record".to_owned())],
            Some(_) => return Err(Error::invalid("invalid rule kind")),
        };
        let answer = self.get(&RULES, &query).await?;
        let groups = answer
            .data
            .get("groups")
            .and_then(Value::as_array)
            .ok_or_else(malformed)?;
        // Validate every rule before anything is returned.
        let mut items = Vec::new();
        let mut total = 0_usize;
        for group in groups {
            let group = group.as_object().ok_or_else(malformed)?;
            let name = text(group, "name", 1024)?;
            let interval = group.get("interval").map(milliseconds).transpose()?;
            for rule in group
                .get("rules")
                .and_then(Value::as_array)
                .ok_or_else(malformed)?
            {
                let rule = rule.as_object().ok_or_else(malformed)?;
                let record = self::rule(name, interval, rule)?;
                if input
                    .kind
                    .as_deref()
                    .is_some_and(|kind| record["kind"] != kind)
                {
                    continue;
                }
                total += 1;
                if items.len() < RULE_LIMIT {
                    items.push(record);
                }
            }
        }
        let truncated = total > RULE_LIMIT;
        Ok(json!({
            "items": items,
            "complete": !truncated,
            "truncation": if truncated { vec!["rule_limit"] } else { vec![] },
            "next_cursor": null,
            "provenance": self.provenance("prometheus:rules"),
        }))
    }
}

/// One provider rule as a `prometheus-rules` record (§11.2).
fn rule(group: &str, interval: Option<u64>, rule: &Map<String, Value>) -> Result<Value> {
    let kind = text(rule, "type", 16)?;
    let health = text(rule, "health", 16)?;
    if !matches!(kind, "alerting" | "recording") || !matches!(health, "ok" | "err" | "unknown") {
        return Err(malformed());
    }
    let mut record = Map::new();
    record.insert("group".into(), json!(group));
    if let Some(interval) = interval {
        record.insert("group_interval_ms".into(), json!(interval));
    }
    record.insert("name".into(), json!(text(rule, "name", 1024)?));
    record.insert("kind".into(), json!(kind));
    record.insert("query".into(), json!(text(rule, "query", 16384)?));
    record.insert("labels".into(), labels(rule.get("labels"), true)?);
    let annotations = if kind == "alerting" {
        labels(rule.get("annotations"), true)?
    } else {
        json!({})
    };
    record.insert("annotations".into(), annotations);
    record.insert("health".into(), json!(health));
    match rule.get("lastError") {
        None | Some(Value::Null) => {}
        Some(Value::String(error)) if error.is_empty() => {}
        Some(Value::String(error)) => {
            record.insert("last_error".into(), json!(clip(error, NOTE_BYTES)));
        }
        Some(_) => return Err(malformed()),
    }
    if kind == "alerting" {
        let state = text(rule, "state", 16)?;
        if !matches!(state, "inactive" | "pending" | "firing") {
            return Err(malformed());
        }
        record.insert("state".into(), json!(state));
        if let Some(duration) = rule.get("duration") {
            record.insert("duration_ms".into(), json!(milliseconds(duration)?));
        }
        let alerts = rule
            .get("alerts")
            .and_then(Value::as_array)
            .ok_or_else(malformed)?;
        record.insert(
            "active_alerts".into(),
            Value::Number(Number::from(alerts.len())),
        );
    }
    Ok(Value::Object(record))
}

#[async_trait]
impl Adapter for Prometheus {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        connectors_sdk::validate(self.input_schema(operation)?, &input)?;
        match operation {
            "series.query" => self.query(decode(input)?).await,
            "series.query_range" => self.query_range(decode(input)?).await,
            "rules.list" => self.rules(decode(input)?).await,
            _ => Err(Error::new(ErrorCode::NotFound, "operation is not provided")),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn notes_are_clipped_on_a_character_boundary() {
        assert_eq!(super::clip("abc", 2), "ab");
        assert_eq!(super::clip("aé", 2), "a");
        assert_eq!(super::clip("é", 2), "é");
    }

    #[test]
    fn decimal_seconds_become_whole_milliseconds() {
        assert_eq!(super::milliseconds(&json!(300)).unwrap(), 300_000);
        assert_eq!(super::milliseconds(&json!(600.5)).unwrap(), 600_500);
        assert_eq!(super::milliseconds(&json!(0.001)).unwrap(), 1);
        assert!(super::milliseconds(&json!(-1)).is_err());
        assert!(super::milliseconds(&json!("1")).is_err());
    }

    #[test]
    fn a_sample_keeps_its_number_and_string_verbatim() {
        let (_, pair) = super::sample(&json!([1788825600.25, "NaN"]), None).unwrap();
        assert_eq!(pair, json!([1788825600.25, "NaN"]));
        assert!(super::sample(&json!([1788825600, 1]), None).is_err());
        assert!(super::sample(&json!([-1, "1"]), None).is_err());
        assert!(super::sample(&json!([10, "1"]), Some((11, 20))).is_err());
    }
}
