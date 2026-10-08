//! Loki binding of the profiles `logql-range`, `logql-metric` and `loki-labels`
//! (`contracts/logs/v1alpha1/semantics.md` beside this adapter, §§3–5 and 11).
//! Every operation is one GET through `AuthenticatedHttp`; the binding is
//! unpaged and retains nothing between calls (§11.3).
use async_trait::async_trait;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::sync::Arc;

const OPERATIONS: [&str; 3] = ["logs.query_range", "logs.query_metric", "logs.labels"];
const QUERY_RANGE: [&str; 4] = ["loki", "api", "v1", "query_range"];
const QUERY: [&str; 4] = ["loki", "api", "v1", "query"];

/// The widest window any operation selects (§4.3).
pub const WINDOW_NS: u64 = 86_400_000_000_000;
/// Loki's own label-API default window, made explicit (§11.2).
pub const LABEL_WINDOW_NS: u64 = 6 * 3_600_000_000_000;
/// Retained stream groups per log read (§5).
pub const STREAM_GROUPS: usize = 500;
/// Serialized result ceiling (§5), less a reserve for the envelope around the lines.
pub const RESULT_BYTES: usize = 4 * 1024 * 1024;
const ENVELOPE_RESERVE: usize = 64 * 1024;
/// Metric bounds (§11.1).
pub const SERIES: usize = 500;
pub const SAMPLES: usize = 50_000;
pub const POINTS: u64 = 11_000;
/// Label bounds (§11.2).
pub const VALUES: usize = 10_000;
pub const VALUE_BYTES: usize = 4096;

pub struct Loki {
    instance: String,
    descriptor: Descriptor,
    http: Arc<dyn AuthenticatedHttp>,
    clock: fn() -> u64,
}

impl Loki {
    /// `effective_configuration` must admit the explicit empty
    /// `query_scope.required_equalities`; no LogQL parser exists to enforce a
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

    fn now_ns(&self) -> Result<u64> {
        (self.clock)()
            .checked_mul(1_000_000)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or_else(Error::internal)
    }

    fn provenance(&self, profile: &str) -> Value {
        json!({
            "instance": self.instance,
            "profile": profile,
            "observed_at_unix_ms": (self.clock)(),
        })
    }

    /// One GET; the validated `data` member of a success envelope.
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<Value> {
        let response = self.http.get(path, query).await.map_err(|e| match e.code {
            ErrorCode::Timeout => e,
            _ => Error::unavailable(),
        })?;
        envelope(response)
    }
}

/// A provider status as the family's error, never carrying the provider's text.
fn envelope(response: HttpResponse) -> Result<Value> {
    match response.status {
        200 => {}
        400 | 422 => return Err(Error::invalid("Loki refused the query")),
        401 => return Err(Error::new(ErrorCode::Unauthorized, "Loki refused access")),
        403 => return Err(Error::new(ErrorCode::Forbidden, "Loki refused the read")),
        429 => {
            return Err(Error::new(
                ErrorCode::RateLimited,
                "Loki rate limit reached",
            ));
        }
        _ => return Err(Error::unavailable()),
    }
    let Ok(Value::Object(mut body)) = connectors_core::read_json::<Value>(&response.body) else {
        return Err(malformed());
    };
    if body.len() != 2 || body.get("status").and_then(Value::as_str) != Some("success") {
        return Err(malformed());
    }
    body.remove("data").ok_or_else(malformed)
}

fn malformed() -> Error {
    Error::new(ErrorCode::Unavailable, "malformed Loki answer")
}

fn decode<T: for<'de> Deserialize<'de>>(input: Value) -> Result<T> {
    serde_json::from_value(input).map_err(|_| Error::invalid("input does not match the operation"))
}

/// Canonical decimal nanoseconds: `0` or `[1-9][0-9]*`, at most `i64::MAX`.
fn canonical_ns(text: &str) -> Option<u64> {
    let digits = !text.is_empty() && text.len() <= 19 && text.bytes().all(|b| b.is_ascii_digit());
    if !digits || (text.len() > 1 && text.starts_with('0')) {
        return None;
    }
    text.parse::<u64>().ok().filter(|n| *n <= i64::MAX as u64)
}

fn input_ns(text: &str) -> Result<u64> {
    canonical_ns(text).ok_or_else(|| Error::invalid("invalid nanosecond timestamp"))
}

fn window(start: u64, end: u64) -> Result<()> {
    if start >= end || end - start > WINDOW_NS {
        return Err(Error::invalid(
            "the window must have start before end and span at most 24 hours",
        ));
    }
    Ok(())
}

/// Loki labels within the §5 bounds, as a JSON object of strings.
fn labels(value: &Value) -> Result<Value> {
    let map = value.as_object().ok_or_else(malformed)?;
    if map.len() > 128 {
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
    if size > 32 * 1024 {
        return Err(malformed());
    }
    Ok(value.clone())
}

/// The longest UTF-8 prefix within `limit` bytes, and whether it was clipped.
fn clip(line: &str, limit: usize) -> (&str, bool) {
    if line.len() <= limit {
        return (line, false);
    }
    let mut end = limit;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    (&line[..end], true)
}

/// Loki's decimal seconds as exact canonical nanoseconds (§11.1).
fn sample_ns(value: &Value) -> Result<String> {
    let text = match value {
        Value::Number(n) => n.to_string(),
        _ => return Err(malformed()),
    };
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 9
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(malformed());
    }
    let whole: u64 = whole.parse().map_err(|_| malformed())?;
    let fraction: u64 = format!("{fraction:0<9}").parse().map_err(|_| malformed())?;
    whole
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(fraction))
        .filter(|n| *n <= i64::MAX as u64)
        .map(|n| n.to_string())
        .ok_or_else(malformed)
}

/// One `[timestamp, "value"]` pair as a sample.
fn sample(pair: &Value) -> Result<Value> {
    match pair.as_array().map(Vec::as_slice) {
        Some([timestamp, Value::String(value)]) => Ok(json!({
            "timestamp_unix_ns": sample_ns(timestamp)?,
            "value": value,
        })),
        _ => Err(malformed()),
    }
}

/// The `data` object of a query answer: its result type and result.
fn query_data(data: Value) -> Result<(String, Value)> {
    let Value::Object(mut data) = data else {
        return Err(malformed());
    };
    if data
        .keys()
        .any(|k| !matches!(k.as_str(), "resultType" | "result" | "stats"))
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    query: String,
    start_unix_ns: String,
    end_unix_ns: Option<String>,
    direction: Option<String>,
    limit: Option<usize>,
    max_line_bytes: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metric {
    query: String,
    time_unix_ns: Option<String>,
    start_unix_ns: Option<String>,
    end_unix_ns: Option<String>,
    step_seconds: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Labels {
    label: Option<String>,
    start_unix_ns: Option<String>,
    end_unix_ns: Option<String>,
    query: Option<String>,
}

/// One retained occurrence: timestamp, then its private (group, entry) coordinates.
struct Occurrence<'a> {
    timestamp: u64,
    group: usize,
    entry: usize,
    line: &'a str,
}

impl Loki {
    async fn query_range(&self, input: Range) -> Result<Value> {
        let start = input_ns(&input.start_unix_ns)?;
        let end = match &input.end_unix_ns {
            Some(end) => input_ns(end)?,
            None => self.now_ns()?,
        };
        window(start, end)?;
        let backward = match input.direction.as_deref() {
            None | Some("backward") => true,
            Some("forward") => false,
            Some(_) => return Err(Error::invalid("invalid direction")),
        };
        let direction = if backward { "backward" } else { "forward" };
        let limit = input.limit.unwrap_or(1000);
        let max_line_bytes = input.max_line_bytes.unwrap_or(8192);
        if !(1..=1000).contains(&limit) || !(1..=8192).contains(&max_line_bytes) {
            return Err(Error::invalid("limit or max_line_bytes out of range"));
        }
        let data = self
            .get(
                &QUERY_RANGE,
                &[
                    ("query", input.query),
                    ("start", start.to_string()),
                    ("end", end.to_string()),
                    ("direction", direction.into()),
                    ("limit", limit.to_string()),
                ],
            )
            .await?;
        let (kind, result) = query_data(data)?;
        match kind.as_str() {
            "streams" => {}
            "matrix" | "vector" | "scalar" => {
                return Err(Error::invalid(
                    "the query is a metric query; use logs.query_metric",
                ));
            }
            _ => return Err(malformed()),
        }
        let groups = result.as_array().ok_or_else(malformed)?;
        // Validate the whole answer before anything is retained (§4.3).
        let mut stream_labels = Vec::with_capacity(groups.len().min(STREAM_GROUPS));
        let mut occurrences = Vec::new();
        let mut total = 0_usize;
        let mut omitted_entries = 0_usize;
        for (group, value) in groups.iter().enumerate() {
            let object = value.as_object().ok_or_else(malformed)?;
            if object.len() != 2 {
                return Err(malformed());
            }
            let stream = labels(object.get("stream").ok_or_else(malformed)?)?;
            let values = object
                .get("values")
                .and_then(Value::as_array)
                .ok_or_else(malformed)?;
            let mut previous: Option<u64> = None;
            for (entry, tuple) in values.iter().enumerate() {
                let Some([Value::String(timestamp), Value::String(line)]) =
                    tuple.as_array().map(Vec::as_slice)
                else {
                    return Err(malformed());
                };
                let timestamp = canonical_ns(timestamp).ok_or_else(malformed)?;
                if timestamp < start || timestamp >= end {
                    return Err(malformed());
                }
                if let Some(previous) = previous
                    && (if backward {
                        timestamp > previous
                    } else {
                        timestamp < previous
                    })
                {
                    return Err(malformed());
                }
                previous = Some(timestamp);
                total += 1;
                if group < STREAM_GROUPS {
                    occurrences.push(Occurrence {
                        timestamp,
                        group,
                        entry,
                        line,
                    });
                } else {
                    omitted_entries += 1;
                }
            }
            if group < STREAM_GROUPS {
                stream_labels.push(stream);
            }
        }
        if total > limit {
            return Err(malformed());
        }
        if backward {
            occurrences.sort_by(|a, b| {
                b.timestamp
                    .cmp(&a.timestamp)
                    .then((a.group, a.entry).cmp(&(b.group, b.entry)))
            });
        } else {
            occurrences.sort_by_key(|o| (o.timestamp, o.group, o.entry));
        }
        let mut lines = Vec::with_capacity(occurrences.len());
        let mut bytes = 0_usize;
        let mut clipped = false;
        let mut response_dropped = 0_usize;
        for occurrence in &occurrences {
            if response_dropped > 0 {
                response_dropped += 1;
                continue;
            }
            let (line, line_truncated) = clip(occurrence.line, max_line_bytes);
            let value = json!({
                "timestamp_unix_ns": occurrence.timestamp.to_string(),
                "stream": stream_labels[occurrence.group],
                "line": line,
                "line_truncated": line_truncated,
                "redacted": false,
                "source": null,
            });
            let size = serde_json::to_vec(&value)
                .map_err(|_| Error::internal())?
                .len()
                + 1;
            if bytes + size > RESULT_BYTES - ENVELOPE_RESERVE {
                response_dropped = 1;
                continue;
            }
            bytes += size;
            clipped |= line_truncated;
            lines.push(value);
        }
        let provider_limit = total == limit;
        let stream_limit = groups.len() > STREAM_GROUPS;
        let mut causes = Vec::new();
        if provider_limit {
            causes.push("provider_limit");
        }
        if stream_limit {
            causes.push("stream_limit");
        }
        if response_dropped > 0 {
            causes.push("response_bytes");
        }
        let complete = causes.is_empty();
        if clipped {
            causes.push("line_bytes");
        }
        // Saturation leaves unseen totals unknown, never zero (§5).
        let (occurrences_dropped, stream_groups_dropped) = if provider_limit {
            (Value::Null, Value::Null)
        } else {
            (
                json!(omitted_entries + response_dropped),
                json!(groups.len().saturating_sub(STREAM_GROUPS)),
            )
        };
        Ok(json!({
            "lines": lines,
            "selection": {
                "kind": "loki-range",
                "start_unix_ns": start.to_string(),
                "end_unix_ns": end.to_string(),
                "direction": direction,
            },
            "order": if backward { "timestamp-backward" } else { "timestamp-forward" },
            "complete": complete,
            "truncation": {
                "causes": causes,
                "occurrences_dropped": occurrences_dropped,
                "stream_groups_dropped": stream_groups_dropped,
            },
            "next_cursor": null,
            "provenance": {
                "instance": self.instance,
                "resource": "loki:query_range",
                "observed_at_unix_ms": (self.clock)(),
                "source_revision": null,
            },
        }))
    }

    async fn query_metric(&self, input: Metric) -> Result<Value> {
        let (path, query) = match (
            &input.start_unix_ns,
            &input.end_unix_ns,
            input.step_seconds,
            &input.time_unix_ns,
        ) {
            (None, None, None, time) => {
                let time = match time {
                    Some(time) => input_ns(time)?,
                    None => self.now_ns()?,
                };
                (
                    &QUERY,
                    vec![("query", input.query), ("time", time.to_string())],
                )
            }
            (Some(start), Some(end), Some(step), None) => {
                let (start, end) = (input_ns(start)?, input_ns(end)?);
                window(start, end)?;
                if !(1..=86_400).contains(&step)
                    || (end - start) / (step * 1_000_000_000) + 1 > POINTS
                {
                    return Err(Error::invalid(
                        "the step must be 1–86400 seconds and give at most 11000 points",
                    ));
                }
                (
                    &QUERY_RANGE,
                    vec![
                        ("query", input.query),
                        ("start", start.to_string()),
                        ("end", end.to_string()),
                        ("step", format!("{step}s")),
                    ],
                )
            }
            _ => {
                return Err(Error::invalid(
                    "give an instant time, or start, end and step together",
                ));
            }
        };
        let (kind, result) = query_data(self.get(path, &query).await?)?;
        let raw: Vec<(Value, Vec<&Value>)> = match kind.as_str() {
            "streams" => {
                return Err(Error::invalid(
                    "the query is a log query; use logs.query_range",
                ));
            }
            "scalar" => vec![(json!({}), vec![&result])],
            "vector" | "matrix" => {
                let items = result.as_array().ok_or_else(malformed)?;
                let mut series = Vec::with_capacity(items.len());
                for item in items {
                    let object = item.as_object().ok_or_else(malformed)?;
                    if object.len() != 2 {
                        return Err(malformed());
                    }
                    let metric = labels(object.get("metric").ok_or_else(malformed)?)?;
                    let pairs = if kind == "vector" {
                        vec![object.get("value").ok_or_else(malformed)?]
                    } else {
                        object
                            .get("values")
                            .and_then(Value::as_array)
                            .ok_or_else(malformed)?
                            .iter()
                            .collect()
                    };
                    series.push((metric, pairs));
                }
                series
            }
            _ => return Err(malformed()),
        };
        let mut causes = Vec::new();
        if raw.len() > SERIES {
            causes.push("series_limit");
        }
        let mut budget = SAMPLES;
        let mut series = Vec::with_capacity(raw.len().min(SERIES));
        let mut sample_limit = false;
        for (index, (metric, pairs)) in raw.iter().enumerate() {
            // The whole answer is validated, including what is not returned.
            let samples = pairs
                .iter()
                .map(|p| sample(p))
                .collect::<Result<Vec<_>>>()?;
            if index >= SERIES || budget == 0 {
                sample_limit |= index < SERIES;
                continue;
            }
            let kept: Vec<Value> = samples.into_iter().take(budget).collect();
            sample_limit |= kept.len() < pairs.len();
            budget -= kept.len();
            series.push(json!({"labels": metric, "samples": kept}));
        }
        if sample_limit {
            causes.push("sample_limit");
        }
        Ok(json!({
            "result_type": kind,
            "series": series,
            "complete": causes.is_empty(),
            "truncation": causes,
            "provenance": self.provenance("logql-metric"),
        }))
    }

    async fn labels(&self, input: Labels) -> Result<Value> {
        let end = match &input.end_unix_ns {
            Some(end) => input_ns(end)?,
            None => self.now_ns()?,
        };
        let start = match &input.start_unix_ns {
            Some(start) => input_ns(start)?,
            None => end.saturating_sub(LABEL_WINDOW_NS),
        };
        window(start, end)?;
        if let Some(label) = &input.label {
            let mut bytes = label.bytes();
            let first = bytes.next();
            if label.len() > 128
                || !first.is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err(Error::invalid("invalid label name"));
            }
        }
        let mut query = vec![("start", start.to_string()), ("end", end.to_string())];
        if let Some(selector) = input.query {
            query.push(("query", selector));
        }
        let data = match &input.label {
            None => self.get(&["loki", "api", "v1", "labels"], &query).await?,
            Some(label) => {
                self.get(&["loki", "api", "v1", "label", label, "values"], &query)
                    .await?
            }
        };
        let data = data.as_array().ok_or_else(malformed)?;
        let mut values = Vec::with_capacity(data.len().min(VALUES));
        for value in data {
            let value = value.as_str().ok_or_else(malformed)?;
            if value.len() > VALUE_BYTES {
                return Err(malformed());
            }
            if values.len() < VALUES {
                values.push(Value::from(value));
            }
        }
        let truncated = data.len() > VALUES;
        let mut answer = Map::new();
        answer.insert("label".into(), json!(input.label));
        answer.insert("values".into(), Value::Array(values));
        answer.insert("complete".into(), json!(!truncated));
        answer.insert(
            "truncation".into(),
            json!(if truncated {
                vec!["value_limit"]
            } else {
                vec![]
            }),
        );
        answer.insert("provenance".into(), self.provenance("loki-labels"));
        Ok(Value::Object(answer))
    }
}

#[async_trait]
impl Adapter for Loki {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        connectors_sdk::validate(self.input_schema(operation)?, &input)?;
        match operation {
            "logs.query_range" => self.query_range(decode(input)?).await,
            "logs.query_metric" => self.query_metric(decode(input)?).await,
            "logs.labels" => self.labels(decode(input)?).await,
            _ => Err(Error::new(ErrorCode::NotFound, "operation is not provided")),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nanoseconds_are_canonical_and_bounded() {
        use super::canonical_ns;
        assert_eq!(canonical_ns("0"), Some(0));
        assert_eq!(
            canonical_ns("9223372036854775807"),
            Some(9_223_372_036_854_775_807)
        );
        for bad in [
            "",
            "00",
            "01",
            "-1",
            "+1",
            " 1",
            "1.0",
            "1e9",
            "9223372036854775808",
        ] {
            assert_eq!(canonical_ns(bad), None, "{bad}");
        }
    }

    #[test]
    fn decimal_seconds_become_exact_nanoseconds() {
        use super::sample_ns;
        use serde_json::json;
        assert_eq!(sample_ns(&json!(1)).unwrap(), "1000000000");
        assert_eq!(sample_ns(&json!(1.5)).unwrap(), "1500000000");
        assert!(sample_ns(&json!(-1)).is_err());
        assert!(sample_ns(&json!("1")).is_err());
    }
}
