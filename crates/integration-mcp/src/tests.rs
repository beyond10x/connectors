use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};

use b10x_mcp_types::{ConnectionId, ToolDescriptor, LEGACY_PROTOCOL_VERSION};
use connector_secrets::{MemoryStore, Secret};
use domain::GrantRisk;
use protocol::operation::OwnerContext;
use service::{
    EgressHttpResponse, EgressTransportError, EgressWebSocket, OperationDeployment,
    ProviderIdentity,
};

const ENDPOINT: &str = "https://mcp.example.test/mcp";
const REMOTE_TOOL: &str = "remote_echo";
const OPERATION: &str = "example.echo.read";

struct ScriptedEgress {
    bearer_uses: AtomicUsize,
}

#[async_trait]
impl EgressTransport for ScriptedEgress {
    async fn execute(
        &self,
        authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        assert_eq!(authority_ref, "endpoint:mcp:example");
        assert_eq!(request.request.url, ENDPOINT);
        assert_eq!(
            request
                .request
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer test-token")
        );
        self.bearer_uses.fetch_add(1, Ordering::Relaxed);
        if request.request.method == "DELETE" {
            return Ok(response(204, None, Vec::new()));
        }
        let body = request.request.body.expect("MCP POST has JSON");
        let value: Value = serde_json::from_str(&body).unwrap();
        let Some(method) = value.get("method").and_then(Value::as_str) else {
            return Ok(response(202, None, Vec::new()));
        };
        let id = value.get("id").cloned().unwrap_or(Value::Null);
        match method {
            "server/discover" => Ok(response(400, Some("text/plain"), b"unsupported".to_vec())),
            "initialize" => Ok(json_response(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": LEGACY_PROTOCOL_VERSION,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "synthetic", "version": "1.0.0"}
                }
            }))),
            "notifications/initialized" => Ok(response(202, None, Vec::new())),
            "tools/list" => Ok(json_response(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"tools": [tool_raw()]}
            }))),
            "tools/call" => Ok(json_response(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": "from remote"}],
                    "isError": false
                }
            }))),
            _ => Ok(json_response(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "unknown"}
            }))),
        }
    }

    async fn connect_websocket(
        &self,
        _authority_ref: &str,
        _url: String,
        _maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

fn response(status: u16, content_type: Option<&str>, body: Vec<u8>) -> EgressHttpResponse {
    let mut headers = BTreeMap::new();
    if let Some(content_type) = content_type {
        headers.insert("content-type".to_owned(), content_type.to_owned());
    }
    EgressHttpResponse {
        status,
        headers,
        body,
    }
}

fn json_response(value: Value) -> EgressHttpResponse {
    response(
        200,
        Some("application/json"),
        serde_json::to_vec(&value).unwrap(),
    )
}

fn tool_raw() -> Value {
    json!({
        "name": REMOTE_TOOL,
        "description": "untrusted server prose",
        "inputSchema": {
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"],
            "additionalProperties": false
        }
    })
}

fn profile() -> McpServiceProfile {
    let descriptor = ToolDescriptor::from_raw(tool_raw(), connector_limits()).unwrap();
    let snapshot = ToolSnapshot::new(
        ConnectionId::new("reviewed_example").unwrap(),
        LEGACY_PROTOCOL_VERSION,
        vec![descriptor],
        connector_limits(),
    )
    .unwrap();
    McpServiceProfile {
        contract: PROFILE_CONTRACT.to_owned(),
        service_ref: "service:mcp:example".to_owned(),
        connection_ref: "connection:mcp:example".to_owned(),
        connection_label: "Reviewed example MCP".to_owned(),
        provider: ReviewedProvider {
            display_name: "Example MCP".to_owned(),
            description: "Reviewed tools from the example MCP endpoint".to_owned(),
        },
        snapshot,
        operations: vec![ReviewedOperation {
            remote_tool: REMOTE_TOOL.to_owned(),
            operation_ref: OPERATION.to_owned(),
            title: "Echo reviewed text".to_owned(),
            description: "Returns text through the reviewed MCP operation".to_owned(),
            effect: EffectClass::ReadOnly,
        }],
    }
}

fn deployment() -> ServiceDeployment {
    ServiceDeployment {
        service_ref: "service:mcp:example".to_owned(),
        provider: ProviderIdentity {
            provider_ref: "provider:mcp:example".to_owned(),
            authority: "test.example.mcp".to_owned(),
        },
        operations: BTreeMap::from([(
            OPERATION.to_owned(),
            OperationDeployment {
                expose: true,
                risk: GrantRisk::Low,
                approval: ApprovalPosture::NotRequired,
                endpoint_bindings: BTreeMap::from([(
                    ENDPOINT_BINDING.to_owned(),
                    "endpoint:mcp:example".to_owned(),
                )]),
                credential_bindings: BTreeMap::from([(
                    BEARER_BINDING.to_owned(),
                    "credential:mcp:example".to_owned(),
                )]),
                grant_refs: BTreeSet::from(["grant:mcp:example-read".to_owned()]),
            },
        )]),
    }
}

fn context() -> PrincipalContext {
    PrincipalContext::local(&OwnerContext {
        tenant_id: "tenant-test".to_owned(),
        agent_id: "agent-test".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot-test".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    })
    .unwrap()
}

#[tokio::test]
async fn frozen_reviewed_tools_cross_connector_custody_and_egress() {
    let credential_ref =
        CredentialRef::new("tenant-test", "test.example.mcp", "default", "bearer").unwrap();
    let store = Arc::new(MemoryStore::new());
    store
        .put(&credential_ref, &Secret::new("test-token"))
        .await
        .unwrap();
    let egress = Arc::new(ScriptedEgress {
        bearer_uses: AtomicUsize::new(0),
    });
    let factory = McpServiceFactory::prepare(
        profile(),
        McpRuntimeBinding {
            endpoint: ENDPOINT.to_owned(),
            endpoint_binding_ref: "endpoint:mcp:example".to_owned(),
            bearer: Some(McpBearerBinding {
                deployment_ref: "credential:mcp:example".to_owned(),
                credential_ref,
            }),
        },
        Arc::clone(&store) as Arc<dyn SecretStore>,
        Arc::clone(&egress) as Arc<dyn EgressTransport>,
    )
    .await
    .unwrap();

    let manifest = factory.manifest();
    assert_eq!(manifest.operations[0].title, "Echo reviewed text");
    assert_ne!(manifest.operations[0].description, "untrusted server prose");
    let dispatch = factory.bind(&deployment()).await.unwrap();
    let (backend, operations) = dispatch.into_parts();
    assert_eq!(operations, BTreeSet::from([OPERATION.to_owned()]));

    let described = backend
        .handle(
            &context(),
            OperationRequest::Describe(DescribeRequest {
                operation_ref: OPERATION.to_owned(),
            }),
        )
        .await
        .unwrap();
    let OperationResult::Describe(description) = described else {
        panic!("expected description");
    };
    assert_eq!(description.title, "Echo reviewed text");

    let invoked = backend
        .handle(
            &context(),
            OperationRequest::Invoke(InvokeRequest {
                operation_ref: OPERATION.to_owned(),
                connection_ref: "connection:mcp:example".to_owned(),
                description_ref: description.description_ref,
                input: json!({"text": "hello"}),
                approval_evidence_ref: None,
            }),
        )
        .await
        .unwrap();
    let OperationResult::Invoke(result) = invoked else {
        panic!("expected invocation");
    };
    assert_eq!(result.output["content"][0]["text"], "from remote");
    assert!(egress.bearer_uses.load(Ordering::Relaxed) >= 4);
}

#[tokio::test]
async fn changed_live_snapshot_is_refused_before_a_factory_exists() {
    let mut changed = profile();
    changed.snapshot.sha256 = "0".repeat(64);
    let store = Arc::new(MemoryStore::new());
    let egress = Arc::new(ScriptedEgress {
        bearer_uses: AtomicUsize::new(0),
    });
    let error = McpServiceFactory::prepare(
        changed,
        McpRuntimeBinding {
            endpoint: ENDPOINT.to_owned(),
            endpoint_binding_ref: "endpoint:mcp:example".to_owned(),
            bearer: None,
        },
        store as Arc<dyn SecretStore>,
        egress as Arc<dyn EgressTransport>,
    )
    .await
    .err();
    assert_eq!(error, Some(McpIntegrationError::InvalidProfile));
}
