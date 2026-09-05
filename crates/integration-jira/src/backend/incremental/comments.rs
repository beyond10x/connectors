//! Comment pages preserve visibility evidence while removing person records.
use super::*;

pub(super) fn output_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["comments","start_at","max_results","total","is_last","next_start_at"],"properties":{
        "comments":{"type":"array","maxItems":100,"items":{"type":"object","additionalProperties":false,"required":["id","body","created","updated","visibility","jsd_public"],"properties":{
            "id":{"type":"string"},"body":{"type":"string"},"created":{"type":"string"},"updated":{"type":"string"},"jsd_public":{"type":["boolean","null"]},
            "visibility":{"type":["object","null"],"additionalProperties":false,"required":["type"],"properties":{"type":{"enum":["role","group"]},"value":{"type":"string"},"identifier":{"type":"string"}}}
        }}},"start_at":{"type":"integer","minimum":0},"max_results":{"type":"integer","minimum":1,"maximum":100},"total":{"type":"integer","minimum":0},"is_last":{"type":"boolean"},"next_start_at":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64}
    }})
}

pub(super) fn project(input: &Value, payload: &Value) -> Result<Value, OperationError> {
    let fail = || operation_protocol();
    let integer = |key: &str| {
        payload[key]
            .as_u64()
            .filter(|v| *v <= u32::MAX as u64)
            .ok_or_else(fail)
    };
    let start = integer("startAt")?;
    let size = integer("maxResults")?;
    let total = integer("total")?;
    let raw = payload["comments"].as_array().ok_or_else(fail)?;
    let limit = input["limit"].as_u64().ok_or_else(fail)?;
    let consumed = start.checked_add(raw.len() as u64).ok_or_else(fail)?;
    let last = consumed >= total;
    if start != input["start_at"].as_u64().unwrap_or(0)
        || size == 0
        || size > limit
        || raw.len() as u64 > size
        || (!last && raw.is_empty())
        || consumed > u32::MAX as u64
    {
        return Err(fail());
    }
    let text = |v: &Value, max: usize| -> Result<Value, OperationError> {
        v.as_str()
            .filter(|s| s.len() <= max)
            .map(|s| json!(s))
            .ok_or_else(fail)
    };
    let mut comments = Vec::new();
    for value in raw {
        let mut visibility = Value::Null;
        if let Some(source) = value.get("visibility").filter(|v| !v.is_null()) {
            if !matches!(source["type"].as_str(), Some("role" | "group")) {
                return Err(fail());
            }
            visibility = json!({"type":source["type"]});
            for key in ["value", "identifier"] {
                if let Some(v) = source.get(key) {
                    visibility[key] = text(v, 256)?;
                }
            }
        }
        let public = value.get("jsdPublic").cloned().unwrap_or(Value::Null);
        if !(public.is_null() || public.is_boolean()) {
            return Err(fail());
        }
        for field in ["id", "created", "updated"] {
            if value[field].as_str().is_none_or(str::is_empty) {
                return Err(fail());
            }
        }
        comments.push(json!({"id":text(&value["id"],64)?,"body":text(&value["body"],32768)?,"created":text(&value["created"],64)?,"updated":text(&value["updated"],64)?,"visibility":visibility,"jsd_public":public}));
    }
    let result = json!({"comments":comments,"start_at":start,"max_results":size,"total":total,"is_last":last,"next_start_at":if last {None}else{Some(consumed)}});
    if !jsonschema::validator_for(&output_schema())
        .map_err(|_| fail())?
        .is_valid(&result)
    {
        return Err(fail());
    }
    Ok(result)
}
