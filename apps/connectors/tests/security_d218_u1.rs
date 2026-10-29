//! story:connectors-do-short-invocation, security review of wave D218 unit U1:
//! the connection `connectors invoke <connector>[/<instance>] <operation>`
//! selects when the command line names none.
//!
//! The approval record of the wave decides it: "The connection is the
//! instance's single `ready` connection. When none or several are ready, the
//! invoke is refused by `connection_unresolved`, listing each connection with
//! its state; `--connection` chooses one." A connection whose validation
//! evidence has lapsed is observed as `pending` (`connections list` reports
//! it so), so an invoke that names no connection must refuse it, naming a
//! repair, and must not reach the adapter.
//!
//! The case runs the production CLI, owner and an adapter child in a jail
//! against the read-only `mounted_secret` binding (`support/mounted.rs`);
//! Linux only, as that binding is. The fixture profile's validation evidence
//! lives 60 seconds, so the case waits for it to lapse.
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

    /// story:connectors-do-short-invocation: an instance whose one connection
    /// is observed `pending`, because its validation evidence lapsed, has no
    /// `ready` connection. `invoke` without `--connection` is then
    /// `connection_unresolved` naming a repair and listing the connection with
    /// its state, and nothing is invoked; the decided rule admits only a
    /// `ready` connection, and the instance's state `pending` is what the
    /// listing and the refusal show the operator.
    #[test]
    fn a_connection_observed_pending_is_never_selected_without_naming_it() {
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        configure(&container);
        let init = success(&container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
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
        // The fixture profile's evidence lives 60 s; wait until the listing
        // reports the one connection `pending`.
        let until = Instant::now() + Duration::from_secs(120);
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
        let output = container.run(&["invoke", "fixture", READ, "--input-json", "{}"]);
        let stderr: Value = serde_json::from_slice(&output.stderr).unwrap_or(Value::Null);
        assert_eq!(
            (output.status.code(), stderr["error"]["data"].clone()),
            (
                Some(1),
                json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                    "next_action":"repair_connection",
                    "connections":[{"connection":connection,"state":"pending"}]})
            ),
            "story:connectors-do-short-invocation: a connection observed pending was \
             selected without --connection; stdout {} stderr {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            container.seen("invocations").len(),
            invoked,
            "story:connectors-do-short-invocation: the adapter was invoked"
        );
    }
}
