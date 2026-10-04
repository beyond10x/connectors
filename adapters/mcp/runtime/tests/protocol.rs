use connectors_mcp::protocol::{LEGACY, PRIMARY, Protocol, Route};
use serde_json::{Value, json};

fn modern(method: &str, version: &str) -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":method,"params":{"_meta":{
        "io.modelcontextprotocol/protocolVersion":version,
        "io.modelcontextprotocol/clientCapabilities":{}}}})
}
#[test]
fn every_primary_request_checks_version_and_capabilities() {
    let mut protocol = Protocol::default();
    assert!(matches!(
        protocol.prepare(&modern("tools/list", PRIMARY)),
        Ok(Route::Work { primary: true })
    ));
    assert_eq!(
        protocol
            .prepare(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .err()
            .unwrap()["code"],
        -32602
    );
    assert_eq!(
        protocol
            .prepare(&modern("tools/list", "future"))
            .err()
            .unwrap(),
        json!({"code":-32022,"message":"Unsupported protocol version","data":{"supported":[PRIMARY,LEGACY],"requested":"future"}})
    );
    let Route::Reply { result, close } = protocol
        .prepare(&modern("server/discover", PRIMARY))
        .unwrap_or_else(|e| panic!("{e}"))
    else {
        panic!("discovery reply")
    };
    assert!(!close);
    assert_eq!(result["capabilities"], json!({"tools":{}}));
    assert_eq!(result["ttlMs"], 0);
    assert_eq!(result["cacheScope"], "private");
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "connectors"
    );
    assert_eq!(
        protocol
            .prepare(&modern("resources/list", PRIMARY))
            .err()
            .unwrap()["code"],
        -32601
    );
}
#[test]
fn legacy_handshake_gates_readiness_and_never_reinitializes() {
    let mut protocol = Protocol::default();
    let initialize = json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":LEGACY,"capabilities":{},"clientInfo":{"name":"peer","version":"1"}}});
    assert!(matches!(
        protocol.prepare(&initialize),
        Ok(Route::Reply { close: false, .. })
    ));
    let list = json!({"jsonrpc":"2.0","id":2,"method":"tools/list"});
    assert_eq!(protocol.prepare(&list).err().unwrap()["code"], -32600);
    assert!(matches!(
        protocol.prepare(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})),
        Ok(Route::Notification)
    ));
    assert!(matches!(
        protocol.prepare(&list),
        Ok(Route::Work { primary: false })
    ));
    assert_eq!(protocol.prepare(&initialize).err().unwrap()["code"], -32600);
}
