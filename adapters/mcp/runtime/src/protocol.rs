//! The selected unary tools protocol. Admission and execution remain host ports.
use launch_types::ConnectorsMcpProtocolLegacySessionPhase as LegacyPhase;
use serde_json::{Value, json};

pub const PRIMARY: &str = "2026-07-28";
pub const LEGACY: &str = "2025-11-25";
pub const REVISION: &str = "io.beyond10x.connectors/revision";
const VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";

#[derive(Default)]
pub struct Protocol {
    legacy: Option<LegacyPhase>,
    modern: bool,
}
/// Routing decisions, not a second session lifecycle model.
pub enum Route {
    Notification,
    Reply { result: Value, close: bool },
    Work { primary: bool },
}
pub fn error(code: i64, message: &str) -> Value {
    json!({"code":code,"message":message})
}
pub fn request_id(value: &Value) -> Option<&Value> {
    value
        .get("id")
        .filter(|id| (id.is_string() || id.is_number()) && id.to_string().len() <= 1024)
}
pub fn complete(mut result: Value, primary: bool, cacheable: bool) -> Value {
    if primary {
        result["resultType"] = json!("complete");
        if cacheable {
            result["ttlMs"] = json!(0);
            result["cacheScope"] = json!("private");
        }
    }
    result
}
impl Protocol {
    pub fn prepare(&mut self, value: &Value) -> Result<Route, Value> {
        let invalid = || error(-32600, "Invalid request");
        let params_error = || error(-32602, "Invalid params");
        let object = value.as_object().ok_or_else(invalid)?;
        if value["jsonrpc"] != "2.0"
            || !value["method"].is_string()
            || object
                .keys()
                .any(|key| !matches!(key.as_str(), "jsonrpc" | "id" | "method" | "params"))
            || (object.contains_key("id") && request_id(value).is_none())
        {
            return Err(invalid());
        }
        let method = value["method"].as_str().ok_or_else(invalid)?;
        if !object.contains_key("id") {
            if method == "notifications/initialized" && matches!(self.legacy, Some(LegacyPhase::V0))
            {
                self.legacy = Some(LegacyPhase::V1);
            }
            return Ok(Route::Notification);
        }
        if value.get("params").is_some_and(|p| !p.is_object()) {
            return Err(params_error());
        }
        let params = &value["params"];
        if method == "initialize" {
            if self.modern || self.legacy.is_some() {
                return Err(invalid());
            }
            let version = params["protocolVersion"]
                .as_str()
                .ok_or_else(params_error)?;
            if !params["capabilities"].is_object()
                || !params["clientInfo"]["name"].is_string()
                || !params["clientInfo"]["version"].is_string()
            {
                return Err(params_error());
            }
            let close = version != LEGACY;
            self.legacy = Some(if close {
                LegacyPhase::V2
            } else {
                LegacyPhase::V0
            });
            return Ok(Route::Reply {
                close,
                result: json!({"protocolVersion":LEGACY,
                "capabilities":{"tools":{}},"serverInfo":{"name":"connectors","version":env!("CARGO_PKG_VERSION")}}),
            });
        }
        let primary = match self.legacy {
            Some(LegacyPhase::V1) => false,
            Some(_) => return Err(invalid()),
            None => {
                let version = params["_meta"][VERSION].as_str().ok_or_else(params_error)?;
                if ![PRIMARY, LEGACY].contains(&version) {
                    return Err(
                        json!({"code":-32022,"message":"Unsupported protocol version",
                        "data":{"supported":[PRIMARY,LEGACY],"requested":version}}),
                    );
                }
                if version != PRIMARY || !params["_meta"][CAPABILITIES].is_object() {
                    return Err(params_error());
                }
                self.modern = true;
                true
            }
        };
        match method {
            "server/discover" if primary => Ok(Route::Reply {
                close: false,
                result: complete(
                    json!({"supportedVersions":[PRIMARY,LEGACY],"capabilities":{"tools":{}},
                        "_meta":{"io.modelcontextprotocol/serverInfo":{"name":"connectors","version":env!("CARGO_PKG_VERSION")}}}),
                    true,
                    true,
                ),
            }),
            "ping" => Ok(Route::Reply {
                close: false,
                result: complete(json!({}), primary, false),
            }),
            "tools/list" | "tools/call" => Ok(Route::Work { primary }),
            _ => Err(error(-32601, "Method not found")),
        }
    }
}
