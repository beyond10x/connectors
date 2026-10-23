//! Result projection from the protected owner's already admitted service reply.
//! These codecs supply neither admission nor capability eligibility. In particular,
//! only a resolved tool execution uses the tool-error channel.
use serde_json::{Value, json};
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Envelope,
    Capacity,
    Identifier,
    ServiceError,
}

pub struct Envelope(Value);
impl Envelope {
    /// Decode a protected owner reply losslessly. Check the service envelope's
    /// shell; the owner remains responsible for admitting its business payload
    /// and optional mutation/source observations before releasing these bytes.
    pub fn from_owner(bytes: &[u8], max_bytes: usize) -> Result<Self, Error> {
        if bytes.len() > max_bytes {
            return Err(Error::Capacity);
        }
        let value = crate::json::decode(bytes, 68).map_err(|_| Error::Json)?;
        let object = value.as_object().ok_or(Error::Envelope)?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "version"
                    | "request_id"
                    | "status"
                    | "result"
                    | "error"
                    | "audit_ref"
                    | "audit_status"
                    | "source_audit"
                    | "mutation"
            )
        }) || value["version"] != "v1alpha2"
            || !object.contains_key("request_id")
            || !(value["request_id"].is_null() || value["request_id"].is_string())
            || !audit(&value)
        {
            return Err(Error::Envelope);
        }
        if let Some(source) = object.get("source_audit") {
            let source_object = source.as_object().ok_or(Error::Envelope)?;
            if source_object.len() != 3 || !nonempty(&source["instance"]) || !audit(source) {
                return Err(Error::Envelope);
            }
        }
        if object
            .get("mutation")
            .is_some_and(|value| !value.is_object())
        {
            return Err(Error::Envelope);
        }
        match value["status"].as_str() {
            Some("success") if object.contains_key("result") && !object.contains_key("error") => {}
            Some("error") if !object.contains_key("result") => {
                let error = value["error"].as_object().ok_or(Error::Envelope)?;
                if error
                    .keys()
                    .any(|key| !matches!(key.as_str(), "code" | "message" | "retry_after_seconds"))
                    || !nonempty(&value["error"]["code"])
                    || !value["error"]["message"]
                        .as_str()
                        .is_some_and(|s| s.len() <= 512)
                    || error
                        .get("retry_after_seconds")
                        .is_some_and(|v| v.as_u64().is_none())
                {
                    return Err(Error::Envelope);
                }
            }
            _ => return Err(Error::Envelope),
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &Value {
        &self.0
    }

    /// Preserve the complete service reply in both required tool representations.
    /// This is for execution after lookup; lookup/refusal errors use rpc_error.
    pub fn resolved_tool(&self, primary: bool) -> Value {
        let mut result = json!({
            "isError": self.0["status"] == "error",
            "structuredContent": self.0,
            "content": [{"type":"text", "text":self.0.to_string()}],
        });
        complete(&mut result, primary, false);
        result
    }

    /// Resources represent a configured empty-input read, never a provider URL.
    /// Family/input admission belongs to the caller; an error is not text content.
    pub fn resource(&self, uri: &str, primary: bool) -> Result<Value, Error> {
        if crate::names::decode_resource_uri(uri).is_none() {
            return Err(Error::Identifier);
        }
        if self.0["status"] != "success" {
            return Err(Error::ServiceError);
        }
        let mut result = json!({"contents":[{"uri":uri, "mimeType":"application/json", "text":self.0.to_string()}]});
        complete(&mut result, primary, true);
        Ok(result)
    }

    /// Project an operation already bound to the declared prompt-message schema.
    /// The caller must have validated that exact schema, including content blocks;
    /// a dataset that happens to contain messages is not eligible by this helper.
    /// Message order/content are data, and are never converted to instructions.
    pub fn declared_prompt(&self, primary: bool) -> Result<Value, Error> {
        if self.0["status"] != "success" {
            return Err(Error::ServiceError);
        }
        let payload = self.0["result"].as_object().ok_or(Error::Envelope)?;
        let messages = payload
            .get("messages")
            .and_then(Value::as_array)
            .ok_or(Error::Envelope)?;
        if messages.iter().any(|message| {
            !matches!(message["role"].as_str(), Some("user" | "assistant"))
                || !message["content"].is_object()
        }) || payload
            .get("description")
            .is_some_and(|description| !description.is_string())
        {
            return Err(Error::Envelope);
        }
        let mut result =
            json!({"messages":messages, "_meta":{"io.beyond10x.connectors/response":self.0}});
        if let Some(description) = payload.get("description") {
            result["description"] = description.clone();
        }
        complete(&mut result, primary, false);
        Ok(result)
    }

    /// The generic safe service error channel used by resources/prompts and by
    /// tool failures before resolution. Never converts a success into an error.
    pub fn rpc_error(&self) -> Result<Value, Error> {
        if self.0["status"] != "error" {
            return Err(Error::Envelope);
        }
        Ok(json!({"code":-32000,"message":"Connectors request refused","data":self.0}))
    }
}

fn nonempty(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.is_empty())
}
fn audit(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let Some(reference) = object.get("audit_ref") else {
        return false;
    };
    match value["audit_status"].as_str() {
        Some("complete" | "incomplete") => nonempty(reference),
        Some("unavailable" | "not_required") => reference.is_null(),
        _ => false,
    }
}
fn complete(result: &mut Value, primary: bool, cacheable: bool) {
    if primary {
        result["resultType"] = json!("complete");
        if cacheable {
            result["ttlMs"] = json!(0);
            result["cacheScope"] = json!("private");
        }
    }
}

pub fn encode_result(id: &Value, result: Value, max_octets: usize) -> Result<Vec<u8>, Error> {
    encode(Some(id), "result", result, max_octets)
}
pub fn encode_error(id: Option<&Value>, error: Value, max_octets: usize) -> Result<Vec<u8>, Error> {
    encode(id, "error", error, max_octets)
}
fn encode(
    id: Option<&Value>,
    field: &str,
    payload: Value,
    max_octets: usize,
) -> Result<Vec<u8>, Error> {
    if id.is_some_and(|id| !id.is_string() && !id.is_number()) {
        return Err(Error::Identifier);
    }
    if !payload.is_object()
        || (field == "error"
            && (payload["code"].as_i64().is_none() || !payload["message"].is_string()))
    {
        return Err(Error::Envelope);
    }
    let mut frame = json!({"jsonrpc":"2.0"});
    if let Some(id) = id {
        frame["id"] = id.clone();
    }
    frame[field] = payload;
    let mut output = Bounded {
        bytes: Vec::new(),
        limit: max_octets.checked_sub(1).ok_or(Error::Capacity)?,
    };
    serde_json::to_writer(&mut output, &frame).map_err(|_| Error::Capacity)?;
    output.bytes.push(b'\n');
    Ok(output.bytes)
}
struct Bounded {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
