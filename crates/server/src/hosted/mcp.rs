//! Stateless MCP transport for the hosted Connectors server (design 14, S-053).
//!
//! `POST {base_path}/mcp` speaks single-message JSON-RPC 2.0 under the MCP revisions
//! 2025-03-26 and 2025-06-18 — no SSE stream, no session id, no batching, no new dependency.
//! The endpoint is an entry to functionality that already exists: the bearer is verified
//! exactly once per HTTP request, before any JSON-RPC processing, and every catalog read,
//! datasource read, and invocation then funnels through the decided halves of the hosted
//! admission handlers (`operation_decided`, `datasource_decided`), so MCP adds zero policy of
//! its own. The catalog invariant beside rule 16 keeps this module and its toolset free of
//! every direct-backend token.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde_json::{json, Map, Value};

use super::{HostedPrincipal, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE};

mod toolset;

/// The MCP revisions this transport speaks, newest last.
const SUPPORTED_REVISIONS: [&str; 2] = ["2025-03-26", "2025-06-18"];
/// Answered when a client proposes a revision outside [`SUPPORTED_REVISIONS`].
const LATEST_REVISION: &str = "2025-06-18";
/// Published server identity. Deliberately unbranded: the platform's published wire ids stay
/// where deployments already carry them, and this surface speaks as b10x.
const SERVER_NAME: &str = "b10x-connectors";

/// One stateless MCP exchange over `POST /mcp`.
pub(super) async fn handle(
    State(state): State<HostedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // The bearer is verified once, before any JSON-RPC processing: an unauthenticated caller
    // learns nothing about the frame grammar, and every seam call below runs under this one
    // verified principal.
    let Some(credential) = super::bearer(&headers) else {
        return super::error(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return super::error(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return super::error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    let Ok(message) = serde_json::from_slice::<Value>(&body) else {
        return rpc_error(
            StatusCode::BAD_REQUEST,
            Value::Null,
            -32700,
            "the frame is not JSON",
        );
    };
    let Some(message) = message.as_object() else {
        // Arrays included: batching left the protocol in 2025-06-18, and this transport
        // refuses it under both admitted revisions rather than answer half a batch.
        return rpc_error(
            StatusCode::BAD_REQUEST,
            Value::Null,
            -32600,
            "one JSON-RPC request object per frame",
        );
    };
    let id = message.get("id").cloned();
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return rpc_error(
            StatusCode::BAD_REQUEST,
            id.unwrap_or(Value::Null),
            -32600,
            "jsonrpc must be \"2.0\"",
        );
    }
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A frame without a method is a client-to-server response; a stateless server issued
        // no request it could answer, so the frame is acknowledged and dropped.
        return StatusCode::ACCEPTED.into_response();
    };
    let Some(id) = id.filter(|_| !method.starts_with("notifications/")) else {
        // Notifications expect no response body.
        return StatusCode::ACCEPTED.into_response();
    };
    if !(id.is_string() || id.is_number()) {
        return rpc_error(
            StatusCode::BAD_REQUEST,
            Value::Null,
            -32600,
            "id must be a string or a number",
        );
    }
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => rpc_result(&id, initialize_result(&params)),
        "ping" => rpc_result(&id, json!({})),
        "tools/list" => rpc_result(&id, json!({ "tools": toolset::meta_tools() })),
        "tools/call" => tools_call(&state, &principal, id, &params).await,
        _ => rpc_error(StatusCode::OK, id, -32601, "method not found"),
    }
}

/// The `initialize` result: echo an admitted revision or answer the newest one. The toolset
/// surface is static, so tool-list change notifications are never emitted.
fn initialize_result(params: &Value) -> Value {
    let proposed = params.get("protocolVersion").and_then(Value::as_str);
    let revision = proposed
        .filter(|revision| SUPPORTED_REVISIONS.contains(revision))
        .unwrap_or(LATEST_REVISION);
    json!({
        "protocolVersion": revision,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
    })
}

/// Route one `tools/call` to a meta-tool. Structurally invalid params refuse as JSON-RPC
/// `-32602`; everything decided at the tool level — refused authority, a hidden or unknown
/// projected tool, invalid tool args — is an execution result with `isError: true` and the
/// protocol's own error code in `structuredContent`.
async fn tools_call(
    state: &HostedState,
    principal: &HostedPrincipal,
    id: Value,
    params: &Value,
) -> Response {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return rpc_error(
            StatusCode::OK,
            id,
            -32602,
            "tools/call params carry a tool name",
        );
    };
    let arguments = match params.get("arguments") {
        None => Value::Object(Map::new()),
        Some(arguments @ Value::Object(_)) => arguments.clone(),
        Some(_) => return rpc_error(StatusCode::OK, id, -32602, "arguments must be an object"),
    };
    let request_id = envelope_request_id(&id);
    let called = match name {
        "tool_search" => toolset::tool_search(state, principal, &request_id, &arguments).await,
        "tool_describe" => toolset::tool_describe(state, principal, &request_id, &arguments).await,
        "tool_invoke" => toolset::tool_invoke(state, principal, &request_id, &arguments).await,
        _ => {
            return rpc_error(
                StatusCode::OK,
                id,
                -32602,
                "unknown tool: tools/list names the whole surface",
            );
        }
    };
    match called {
        Ok(result) => rpc_result(&id, result),
        Err(invalid) => rpc_error(StatusCode::OK, id, -32602, &invalid),
    }
}

/// The seam correlation id derived from the JSON-RPC id: ASCII-graphic, bounded, and shared by
/// every envelope one MCP exchange synthesizes.
fn envelope_request_id(id: &Value) -> String {
    let raw = match id {
        Value::String(id) => id.clone(),
        other => other.to_string(),
    };
    let cleaned: String = raw
        .chars()
        .filter(char::is_ascii_graphic)
        .take(120)
        .collect();
    if cleaned.is_empty() {
        "mcp-request".to_owned()
    } else {
        format!("mcp-{cleaned}")
    }
}

fn rpc_result(id: &Value, result: Value) -> Response {
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response()
}

fn rpc_error(status: StatusCode, id: Value, code: i32, message: &str) -> Response {
    (
        status,
        Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        })),
    )
        .into_response()
}
