//! story:connectors-do-short-invocation, security review of wave D218 unit U1:
//! the connection `connectors invoke <connector>[/<instance>] <operation>`
//! selects when the command line names none.
//!
//! The approval record of the wave decides it, as amended (D3): the instance's
//! single `ready` connection; with none ready, its single usable one, whose
//! validation evidence lapsed under the current configuration (observed as
//! `pending`, as an idle connection's is) and is re-checked with the provider
//! before the call; otherwise `connection_unresolved`, listing each
//! connection with its state. So an invoke that names no connection uses an
//! instance's one lapsed connection only after its evidence was re-checked.
//!
//! The case runs the production CLI, owner and an adapter child in a jail
//! against the read-only `mounted_secret` binding (`support/mounted.rs`);
//! Linux only, as that binding is. The fixture's validation evidence is set
//! to live 3 seconds, and the case waits for it to lapse.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;

mod jailed {
    use crate::mounted::*;
    use serde_json::{Value, json};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    /// The adapter child the owner starts filters to this test's exact name.
    pub const ADAPTER_FIXTURE: &str = "jailed::security_d218_u1_adapter_fixture";

    /// The adapter child the owner starts: this binary, filtered to this test.
    /// The ordinary run does nothing.
    #[test]
    fn security_d218_u1_adapter_fixture() {
        serve_when_child();
    }

    /// The fixture adapter configured as instance `fixture/main` (alias
    /// `forge`) of a `connectors-local/5` file, custody the mounted directory.
    fn configure(container: &Container) {
        let uid = connectors_host::local::filesystem::uid();
        let adapter = digest(&fs::read(container.path("/bin-test/adapter")).unwrap());
        let args = serde_json::to_string(&["--exact", ADAPTER_FIXTURE, "--"]).unwrap();
        let table = "connectors.fixture.instances.main";
        let text = format!(
            "format='connectors-local/5'\nowner_uid={uid}\n{}\n[connectors.fixture]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[{table}.permissions]\nprofiles=['{PROFILE}','{OAUTH_PROFILE}']\noperations=['{READ}']\n[{table}.executable]\npath='/bin-test/adapter'\nsha256='{adapter}'\nargs={args}\n",
            Container::mounted()
        );
        let path = container.path(CONFIG);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// story:connectors-do-short-invocation, D3 amended: an instance whose one
    /// connection is observed `pending`, because its validation evidence
    /// lapsed, has no `ready` connection and one usable one. `invoke` without
    /// `--connection` selects it, and the provider re-checks its evidence
    /// before the one call reaches the adapter.
    #[test]
    fn a_connection_observed_pending_is_selected_after_its_evidence_is_rechecked() {
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        configure(&container);
        let init = success(&container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
        container.set_knob("evidence_ms", 3_000);
        let connected = success(&container.connect(PROFILE));
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();
        let state = |container: &Container| -> String {
            let listed = success(&container.run(&[
                "connections",
                "list",
                "--connector",
                "fixture",
                "--instance",
                "main",
            ]));
            listed.to_string()
        };
        assert!(
            state(&container).contains("\"state\":\"ready\""),
            "precondition: the fresh connection is ready: {}",
            state(&container)
        );
        // The evidence lives 3 s; wait until the listing reports the one
        // connection `pending`.
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            let listed = state(&container);
            if listed.contains("\"state\":\"pending\"") {
                break;
            }
            assert!(
                Instant::now() < until,
                "precondition: the connection never read pending: {listed}"
            );
            std::thread::sleep(Duration::from_secs(1));
        }
        let invoked = container.seen("invocations").len();
        let validated = container.seen("validations").len();
        let output = container.run(&["invoke", "fixture", READ, "--input-json", "{}"]);
        let stdout: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
        assert_eq!(
            (
                output.status.code(),
                &stdout["result"]["connector"],
                &stdout["result"]["instance"],
                container.seen("validations").len() - validated,
                container.seen("invocations").len() - invoked,
            ),
            (Some(0), &json!("fixture"), &json!("main"), 1, 1),
            "story:connectors-do-short-invocation: D3 amended: lapsed evidence is re-checked before the call; connection {connection}, stdout {} stderr {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
