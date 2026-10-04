//! Production MCP stdio entry: protocol stdout is never a finite CLI envelope.
use std::process::{Command, Stdio};

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command.env_clear().stdin(Stdio::null());
    command
}

#[test]
fn selected_launch_help_uses_the_native_grammar() {
    let output = command().args(["server", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--transport"), "{help}");
    assert!(help.contains("--config"), "{help}");
    assert!(!help.contains("--output"), "{help}");
}

#[test]
fn owner_refusal_and_invalid_launch_emit_no_protocol_bytes() {
    let root = tempfile::tempdir().unwrap();
    for suffix in [
        vec!["--transport", "stdio"],
        vec![],
        vec!["--transport", "http"],
        vec!["--transport", "stdio", "--output", "json"],
        vec!["--transport", "stdio", "--credential-stdin"],
    ] {
        let output = command()
            .arg("--config")
            .arg(root.path().join("absent.toml"))
            .arg("--state-dir")
            .arg(root.path().join("absent-state"))
            .arg("server")
            .args(suffix)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            output.stdout.is_empty(),
            "CLI envelope leaked to protocol stdout: {output:?}"
        );
        assert!(!output.stderr.is_empty());
        assert!(!root.path().join("absent-state").exists());
    }
}

#[path = "support/mcp/mod.rs"]
mod fixture;
use serde_json::{Value, json};

#[test]
fn native_provider_fixture() {
    let args: Vec<_> = std::env::args().collect();
    if !args.iter().any(|arg| arg == "--connectors-private-fd") {
        return;
    }
    let root = args
        .iter()
        .find_map(|arg| arg.strip_prefix("mcp-root="))
        .unwrap()
        .into();
    connectors_host::local::runtime::serve(3, fixture::Provider { root }).unwrap();
}

/// Actual-process bindings for local-server-admitted-discovery and the tools
/// subset of local-server-typed-results; production owner, custody and child.
#[test]
#[ignore = "requires disposable dbus-daemon and GNOME Secret Service; run explicitly"]
fn production_stdio_lists_and_invokes_an_admitted_operation() {
    let f = fixture::Fixture::new();
    for (count, primary) in [true, false].into_iter().enumerate() {
        let mut peer = fixture::Peer::start(&f, primary);
        let list = peer.request(1, "tools/list", json!({}));
        let tools = list["result"]["tools"].as_array().expect("admitted list");
        assert_eq!(tools.len(), 1, "{list}");
        assert_eq!(f.calls(), count, "listing invoked provider");
        let name = tools[0]["name"].as_str().unwrap();
        let revision = tools[0]["_meta"]["io.beyond10x.connectors/revision"].clone();
        assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
        assert_eq!(
            tools[0]["outputSchema"]["properties"]["result"]["properties"]["observed"]["type"],
            "number"
        );
        let input: Value = serde_json::from_str(r#"{"n":1844674407370955161701}"#).unwrap();
        let result=peer.request(2,"tools/call",json!({"name":name,"arguments":{"input":input},"_meta":{"io.beyond10x.connectors/revision":revision}}));
        assert_eq!(result["result"]["isError"], false, "{result}");
        let envelope = &result["result"]["structuredContent"];
        assert_eq!(envelope["status"], "success", "{result}");
        assert_eq!(envelope["result"]["observed"], input["n"]);
        assert_eq!(envelope["audit_status"], "complete");
        assert!(envelope["audit_ref"].is_string());
        assert!(envelope["request_id"].is_string());
        assert_eq!(
            serde_json::from_str::<Value>(result["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap(),
            *envelope
        );
        assert_eq!(f.calls(), count + 1);
        for (id, op, expected) in [
            (3, "disabled", "forbidden"),
            (4, "hidden", "not_found"),
            (5, "guarded", "not_found"),
            (6, "absent", "not_granted"),
        ] {
            let name = connectors_mcp::names::encode_name(["local", "fixture", op]).unwrap();
            let result=peer.request(id,"tools/call",json!({"name":name,"arguments":{"input":{"n":17}},"_meta":{"io.beyond10x.connectors/revision":revision}}));
            assert_eq!(
                result["error"]["data"]["error"]["code"], expected,
                "{result}"
            );
        }
        assert_eq!(f.calls(), count + 1, "refused operation reached provider");
        peer.finish();
    }
}

/// Process bindings for revision-and-capability, framing-and-loss and the
/// explicit request-cancellation subset of lease-and-cancel.
#[test]
#[ignore = "requires disposable dbus-daemon and GNOME Secret Service; run explicitly"]
fn production_stdio_withdrawal_framing_and_cancellation() {
    let f = fixture::Fixture::new();
    let mut peer = fixture::Peer::start(&f, true);
    peer.send(json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}));
    assert_eq!(peer.recv()["error"]["code"], -32602);
    let list = peer.request(2, "tools/list", json!({}));
    let tool = &list["result"]["tools"][0];
    let name = tool["name"].clone();
    let revision = tool["_meta"]["io.beyond10x.connectors/revision"].clone();
    f.exposures(false);
    let list = peer.request(3, "tools/list", json!({}));
    assert_eq!(list["result"]["tools"], json!([]), "{list}");
    let response=peer.request(4,"tools/call",json!({"name":name,"arguments":{"input":{"n":1}},"_meta":{"io.beyond10x.connectors/revision":revision}}));
    assert_eq!(
        response["error"]["data"]["error"]["code"], "stale_description",
        "{response}"
    );
    assert_eq!(f.calls(), 0);
    peer.raw(b"{broken\n");
    assert_eq!(peer.recv()["error"]["code"], -32700);
    peer.raw(b"[]\n");
    assert_eq!(peer.recv()["error"]["code"], -32600);
    let oversized = vec![b' '; 1_048_577];
    peer.raw(&oversized);
    peer.raw(b"\n");
    assert_eq!(peer.recv()["error"]["code"], -32700);
    let ping = peer.request(5, "ping", json!({}));
    assert_eq!(ping["result"]["resultType"], "complete");
    f.exposures(true);
    let list = peer.request(6, "tools/list", json!({}));
    let revision = list["result"]["tools"][0]["_meta"]["io.beyond10x.connectors/revision"].clone();
    peer.send(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":name,"arguments":{"input":{"n":999}},"_meta":{"io.beyond10x.connectors/revision":revision,"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}));
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while f.calls() == 0 {
        assert!(std::time::Instant::now() < until, "provider not dispatched");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    peer.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":7}}));
    let ping = peer.request(8, "ping", json!({}));
    assert!(ping.get("result").is_some(), "{ping}");
    std::thread::sleep(std::time::Duration::from_millis(2100));
    let ping = peer.request(9, "ping", json!({}));
    assert!(
        ping.get("result").is_some(),
        "cancelled call answered: {ping}"
    );
    assert_eq!(f.calls(), 1, "cancellation replayed the provider call");
    peer.raw(b"{\"jsonrpc\":\"2.0\""); // EOF discards an unfinished frame.
    peer.finish();
}

#[test]
#[ignore = "requires disposable dbus-daemon and GNOME Secret Service; run explicitly"]
fn production_stdio_stopped_session_cannot_renew_after_expiry() {
    let f = fixture::Fixture::new();
    let mut peer = fixture::Peer::start(&f, true);
    assert!(peer.request(1, "ping", json!({})).get("result").is_some());
    // SAFETY: these signals target only the fixture-owned child, still unreaped.
    assert_eq!(
        unsafe { libc::kill(peer.child.id() as i32, libc::SIGSTOP) },
        0
    );
    std::thread::sleep(std::time::Duration::from_millis(2200));
    assert_eq!(
        unsafe { libc::kill(peer.child.id() as i32, libc::SIGCONT) },
        0
    );
    peer.exit(false);
    assert_eq!(f.calls(), 0);
}
