//! Redaction-owning projections of upstream monitoring answers (split from `backend.rs` under
//! the module-size fence: the projection arm is self-contained and shared by the direct and
//! mediated routes).

use protocol::operation::OperationError;
use serde_json::Value;

use crate::errors::*;

pub(crate) fn project_output(operation: &str, raw: &Value) -> Result<Value, OperationError> {
    let projected = match operation {
        monitoring_model::GRAFANA_DATASOURCES_LIST => project_datasources(raw)?,
        monitoring_model::GRAFANA_DASHBOARDS_LIST => project_dashboard_list(raw)?,
        monitoring_model::GRAFANA_DASHBOARD_GET => project_dashboard(raw)?,
        monitoring_model::PROMETHEUS_QUERY_RANGE => project_prometheus(raw)?,
        monitoring_model::LOKI_QUERY_RANGE => project_loki(raw)?,
        monitoring_model::ALERTMANAGER_ALERTS_LIST => project_alerts(raw)?,
        _ => return Err(operation_not_found()),
    };
    if serde_json::to_vec(&projected).map_or(true, |bytes| {
        bytes.len() > protocol::operation::MAX_RESULT_BYTES
    }) {
        return Err(operation_unavailable());
    }
    Ok(projected)
}

fn project_datasources(raw: &Value) -> Result<Value, OperationError> {
    let sources = raw
        .as_array()
        .ok_or_else(operation_invalid)?
        .iter()
        .take(500)
        .filter_map(|source| {
            let object = source.as_object()?;
            let name = bounded_text(object.get("name")?.as_str()?, 256);
            let provider = bounded_text(object.get("type")?.as_str()?, 128);
            (!name.is_empty() && !provider.is_empty()).then(|| {
                serde_json::json!({
                    "name": name,
                    "provider": provider,
                    "status": "available",
                    "callable": matches!(provider.as_str(), "prometheus" | "loki" | "alertmanager"),
                })
            })
        })
        .collect::<Vec<_>>();
    Ok(
        serde_json::json!({"sources": sources, "complete": raw.as_array().is_some_and(|items| items.len() <= 500)}),
    )
}

fn project_dashboard_list(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    let items = object
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(operation_invalid)?;
    let dashboards = items
        .iter()
        .take(500)
        .filter_map(project_dashboard_summary)
        .collect::<Vec<_>>();
    let next_cursor = object
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("continue"))
        .and_then(Value::as_str)
        .map(|value| bounded_text(value, 4096));
    Ok(serde_json::json!({
        "dashboards": dashboards,
        "next_cursor": next_cursor,
        "complete": items.len() <= 500 && next_cursor.is_none(),
    }))
}

fn project_dashboard_summary(value: &Value) -> Option<Value> {
    let object = value.as_object()?;
    let metadata = object.get("metadata")?.as_object()?;
    let spec = object.get("spec")?.as_object()?;
    let uid = bounded_text(metadata.get("name")?.as_str()?, 256);
    let title = bounded_text(spec.get("title")?.as_str()?, 256);
    let tags = spec
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(32)
        .map(|tag| bounded_text(tag, 128))
        .collect::<Vec<_>>();
    Some(serde_json::json!({"uid": uid, "title": title, "tags": tags}))
}

fn project_dashboard(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    let metadata = object
        .get("metadata")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let spec = object
        .get("spec")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let panels = spec
        .get("panels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(500)
        .filter_map(|panel| {
            let panel = panel.as_object()?;
            Some(serde_json::json!({
                "title": bounded_text(panel.get("title").and_then(Value::as_str).unwrap_or("Untitled"), 256),
                "kind": bounded_text(panel.get("type").and_then(Value::as_str).unwrap_or("unknown"), 64),
            }))
        })
        .collect::<Vec<_>>();
    let tags = spec
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(32)
        .map(|tag| bounded_text(tag, 128))
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "uid": bounded_text(metadata.get("name").and_then(Value::as_str).unwrap_or(""), 256),
        "title": bounded_text(spec.get("title").and_then(Value::as_str).unwrap_or("Untitled"), 256),
        "description": redact_text(spec.get("description").and_then(Value::as_str).unwrap_or(""), 1024).0,
        "tags": tags,
        "panels": panels,
        "panels_truncated": spec.get("panels").and_then(Value::as_array).is_some_and(|items| items.len() > 500),
    }))
}

fn project_prometheus(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    if object.get("status").and_then(Value::as_str) != Some("success") {
        return Err(operation_unavailable());
    }
    let data = object
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let result = data
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(operation_invalid)?;
    let series = result
        .iter()
        .take(500)
        .filter_map(|series| {
            let series = series.as_object()?;
            let labels = project_labels(series.get("metric"));
            let samples = series
                .get("values")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(2_000)
                .filter_map(project_sample)
                .collect::<Vec<_>>();
            let instant = series.get("value").and_then(project_sample);
            Some(serde_json::json!({"labels": labels, "samples": samples, "value": instant}))
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "status": "success",
        "result_type": bounded_text(data.get("resultType").and_then(Value::as_str).unwrap_or("unknown"), 32),
        "series": series,
        "truncated": result.len() > 500,
    }))
}

fn project_sample(value: &Value) -> Option<Value> {
    let pair = value.as_array()?;
    (pair.len() == 2).then(|| {
        serde_json::json!({
            "timestamp": pair[0].clone(),
            "value": bounded_text(pair[1].as_str().unwrap_or(""), 128),
        })
    })
}

fn project_loki(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    if object.get("status").and_then(Value::as_str) != Some("success") {
        return Err(operation_unavailable());
    }
    let data = object
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let streams = data
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(operation_invalid)?;
    let mut lines = Vec::new();
    for stream in streams.iter().take(500) {
        let Some(stream) = stream.as_object() else {
            continue;
        };
        let labels = project_labels(stream.get("stream"));
        for pair in stream
            .get("values")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if lines.len() == 1_000 {
                break;
            }
            let Some(pair) = pair.as_array().filter(|pair| pair.len() == 2) else {
                continue;
            };
            let (line, redacted) = redact_text(pair[1].as_str().unwrap_or(""), 8 * 1024);
            lines.push(serde_json::json!({
                "timestamp": pair[0].clone(),
                "labels": labels,
                "line": line,
                "redacted": redacted,
                "truncated": pair[1].as_str().is_some_and(|value| value.len() > 8 * 1024),
            }));
        }
        if lines.len() == 1_000 {
            break;
        }
    }
    Ok(serde_json::json!({
        "result_type": bounded_text(data.get("resultType").and_then(Value::as_str).unwrap_or("streams"), 32),
        "lines": lines,
        "truncated": lines.len() == 1_000 || streams.len() > 500,
    }))
}

fn project_alerts(raw: &Value) -> Result<Value, OperationError> {
    let alerts = raw
        .as_array()
        .ok_or_else(operation_invalid)?
        .iter()
        .take(500)
        .filter_map(|alert| {
            let alert = alert.as_object()?;
            let annotations = alert.get("annotations").and_then(Value::as_object);
            let (summary, summary_redacted) = redact_text(
                annotations
                    .and_then(|values| values.get("summary"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                512,
            );
            let status = alert
                .get("status")
                .and_then(Value::as_object)
                .and_then(|status| status.get("state"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Some(serde_json::json!({
                "labels": project_labels(alert.get("labels")),
                "summary": summary,
                "summary_redacted": summary_redacted,
                "state": bounded_text(status, 64),
                "starts_at": bounded_text(alert.get("startsAt").and_then(Value::as_str).unwrap_or(""), 64),
                "ends_at": bounded_text(alert.get("endsAt").and_then(Value::as_str).unwrap_or(""), 64),
            }))
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "alerts": alerts,
        "complete": raw.as_array().is_some_and(|items| items.len() <= 500),
    }))
}

fn project_labels(value: Option<&Value>) -> Value {
    const ALLOWED: [&str; 12] = [
        "__name__",
        "alertname",
        "cluster",
        "container",
        "deployment",
        "instance",
        "job",
        "namespace",
        "pod",
        "service",
        "severity",
        "status",
    ];
    let labels = value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, _)| ALLOWED.contains(&key.as_str()))
        .filter_map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), Value::String(redact_text(value, 256).0)))
        })
        .collect::<serde_json::Map<_, _>>();
    Value::Object(labels)
}

fn bounded_text(value: &str, maximum: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(maximum)
        .collect()
}

fn redact_text(value: &str, maximum: usize) -> (String, bool) {
    let lowered = value.to_ascii_lowercase();
    let sensitive = [
        "authorization:",
        "bearer ",
        "password=",
        "password:",
        "secret=",
        "secret:",
        "token=",
        "token:",
        "api_key=",
        "api-key:",
        "-----begin private key-----",
    ]
    .iter()
    .any(|pattern| lowered.contains(pattern))
        || value
            .split('.')
            .filter(|segment| segment.len() > 16)
            .count()
            >= 3
            && value.starts_with("eyJ");
    if sensitive {
        ("[redacted]".to_owned(), true)
    } else {
        (bounded_text(value, maximum), false)
    }
}
