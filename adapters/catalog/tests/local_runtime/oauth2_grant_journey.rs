//! `oauth2_grant` through the production CLI, owner, catalog child and
//! disposable Secret Service (story:connect-runs-an-oauth2-grant). One fixture
//! host is the authorization server and the API. `connections connect` and
//! `reconnect` run the grant in the CLI process: the debug-build follow hook
//! (`CONNECTORS_TEST_OAUTH_FOLLOW`) consents where a person would, follows the
//! loopback or pasted-code redirect, or approves the device code. The owner
//! validates the token response it is handed and stores it as the entry
//! `{client_id, access_token, refresh_token, expires_at_ms, scopes}`; a grant
//! that fails stores nothing. Every journey also reads the recorded metadata
//! authority: one `AuthorizationServer` per profile record, and the
//! `Acquisition`'s transitions.
use super::*;
use crate::oauth2_grant::{
    Behaviour, GRANT_ACCESS, GRANT_EXTRA_SCOPE, GRANT_PROFILE, GRANT_SCOPE, GrantServer, Redirect,
    carries_grant_material, declaration, grant_config,
};

const SERVER: &str = "connectors.oauth2.AuthorizationServer";
const PROFILE: &str = "connectors.auth_bindings.AuthProfile";
const ACQUISITION: &str = "connectors.auth_bindings.Acquisition";
const CONNECTION: &str = "connectors.auth_bindings.Connection";
const CUSTODY: &str = "connectors.auth_bindings.CustodyVersion";

struct GrantJourney {
    // Dropped in this order: the owner stops before custody and the fixture.
    cli: Cli,
    _custody: Custody,
    server: GrantServer,
}

impl GrantJourney {
    fn new(behaviour: Behaviour, redirect: Redirect) -> Self {
        let server = GrantServer::new(behaviour);
        server.write_config(&grant_config(
            server.port,
            &server.ca,
            declaration(server.port, redirect),
        ));
        let custody = Custody::new(server.root.path());
        let cli = Cli::new(server.root.path());
        success(cli.run(&["setup", "init"]));
        success(cli.run(&["setup", "check"]));
        let adapter = server.selection();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let configuration = format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.grant]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.grant.permissions]\nprofiles=[{}]\noperations=['project.get']\n[adapters.grant.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&adapter.instance_id),
            q(&adapter.configuration_revision),
            q(GRANT_PROFILE),
            q(adapter.executable.path.to_str().unwrap()),
            q(&adapter.executable.sha256),
            serde_json::to_string(&adapter.executable.args).unwrap()
        );
        private(&cli.paths.config, configuration.as_bytes());
        Self {
            cli,
            _custody: custody,
            server,
        }
    }

    /// Runs `args` with the follow hook and checks that no token, code or
    /// device code reaches either output stream.
    fn follow(&self, args: &[&str]) -> Output {
        let output = self.cli.run_following(args, &self.server.ca);
        assert!(
            !carries_grant_material(&output.stdout),
            "material on stdout"
        );
        assert!(
            !carries_grant_material(&output.stderr),
            "material on stderr"
        );
        output
    }

    fn connect(&self, extra: &[&str]) -> Output {
        let mut args = vec![
            "connections",
            "connect",
            "--adapter",
            "grant",
            "--profile",
            GRANT_PROFILE,
        ];
        args.extend_from_slice(extra);
        self.follow(&args)
    }

    fn reconnect(&self, reference: &str, revision: &str, extra: &[&str]) -> Output {
        let mut args = vec![
            "connections",
            "reconnect",
            "--adapter",
            "grant",
            "--connection",
            reference,
            "--expected-revision",
            revision,
        ];
        args.extend_from_slice(extra);
        self.follow(&args)
    }

    fn describe(&self, reference: &str) -> Value {
        let output = self.cli.run(&[
            "connections",
            "describe",
            "--adapter",
            "grant",
            "--connection",
            reference,
        ]);
        assert!(!carries_grant_material(&output.stdout));
        success(output)["connection"].clone()
    }

    fn read(&self, reference: &str) -> Value {
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "grant",
            "--operation",
            "project.get",
        ]));
        let output = self.cli.run(&[
            "invoke",
            "gitlab/grant",
            "project.get",
            "--connection",
            reference,
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            r#"{"id":"org/project"}"#,
        ]);
        assert!(!carries_grant_material(&output.stdout));
        assert!(!carries_grant_material(&output.stderr));
        success(output)
    }

    /// The configuration, the owner's state and the provider configuration
    /// carry no token at rest: the entry lives in custody only.
    fn assert_no_material_at_rest(&self) {
        let mut written = vec![self.cli.paths.config.clone(), self.server.config.clone()];
        files_under(&self.cli.paths.state, &mut written);
        for path in &written {
            let bytes = fs::read(path).unwrap_or_default();
            assert!(!carries_grant_material(&bytes), "{}", path.display());
        }
    }

    /// One `AuthorizationServer` per recorded profile record of the grant
    /// profile, created once (`DeclareAuthorizationServer`), naming that
    /// record and carrying the declaration the catalog read.
    fn assert_declared_servers(&self, facts: &recorded_state::Facts) {
        let profiles: Vec<_> = facts
            .subjects(PROFILE)
            .into_iter()
            .filter(|row| row.fields["profile_id"] == GRANT_PROFILE)
            .collect();
        assert!(!profiles.is_empty(), "the grant profile was recorded");
        let servers = facts.subjects(SERVER);
        assert_eq!(
            servers.len(),
            profiles.len(),
            "one server per profile record"
        );
        for profile in profiles {
            let record = &profile.fields["profile_record_ref"];
            let declared: Vec<_> = servers
                .iter()
                .filter(|server| &server.fields["profile_record_ref"] == record)
                .collect();
            assert_eq!(declared.len(), 1, "{record}");
            let server = declared[0];
            assert_eq!(server.lifecycle_state, "Declared");
            assert_eq!(
                server.fields["declaration"]["token_url"],
                format!("https://localhost:{}/token", self.server.port)
            );
            assert_eq!(
                server.fields["declaration"]["required_scopes"],
                json!([GRANT_SCOPE])
            );
            let transitions = facts.transitions(server);
            assert_eq!(transitions.len(), 1, "declared once: {transitions:?}");
            assert_eq!(transitions[0].from, None);
            assert_eq!(transitions[0].to, "Declared");
            assert_eq!(
                transitions[0].events,
                vec!["connectors.oauth2.AuthorizationServerDeclared".to_owned()]
            );
        }
    }

    /// The recorded facts once every acquisition of this instance has ended.
    /// A grant that fails in the CLI ends its capture by closing it, and the
    /// owner records that failure while or after the CLI exits.
    fn settled(&self) -> recorded_state::Facts {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let facts = self.cli.recorded();
            let open = facts.subjects(ACQUISITION).into_iter().any(|row| {
                row.fields["instance_id"] == "fixture-grant"
                    && !matches!(
                        row.lifecycle_state.as_str(),
                        "Completed" | "Failed" | "Expired"
                    )
            });
            if !open || std::time::Instant::now() >= deadline {
                return facts;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// The states each recorded acquisition of this instance passed through,
    /// in the order they were begun.
    fn acquisitions(&self, facts: &recorded_state::Facts) -> Vec<Vec<String>> {
        let mut rows: Vec<_> = facts
            .subjects(ACQUISITION)
            .into_iter()
            .filter(|row| row.fields["instance_id"] == "fixture-grant")
            .collect();
        rows.sort_by(|a, b| {
            a.fields["created_at"]
                .as_str()
                .cmp(&b.fields["created_at"].as_str())
        });
        rows.into_iter()
            .map(|row| {
                let transitions = facts.transitions(row);
                assert_eq!(transitions[0].from, None);
                transitions.iter().map(|step| step.to.clone()).collect()
            })
            .collect()
    }
}

/// An acquisition that began, was consumed and completed.
#[track_caller]
fn completed(states: &[String]) {
    assert_eq!(
        states.first().map(String::as_str),
        Some("Pending"),
        "{states:?}"
    );
    assert_eq!(
        states.last().map(String::as_str),
        Some("Completed"),
        "{states:?}"
    );
    assert!(
        states[1..states.len() - 1]
            .iter()
            .all(|state| state == "Completing"),
        "{states:?}"
    );
    assert!(states.len() >= 3, "{states:?}");
}

/// An acquisition that began and failed, stored nothing.
#[track_caller]
fn failed(states: &[String]) {
    assert_eq!(
        states.first().map(String::as_str),
        Some("Pending"),
        "{states:?}"
    );
    assert_eq!(
        states.last().map(String::as_str),
        Some("Failed"),
        "{states:?}"
    );
}

/// No connection of the instance was published.
#[track_caller]
fn nothing_published(facts: &recorded_state::Facts) {
    for row in facts.subjects(CONNECTION) {
        assert_eq!(row.fields["published"], false, "{:?}", row.fields);
    }
}

/// The connection is ready, and its description shows the granted scopes and
/// the access token's expiry, never a token.
#[track_caller]
fn assert_granted(described: &Value, reference: &str) {
    assert_eq!(described["summary"]["connection"], reference);
    assert_eq!(described["summary"]["state"], "ready", "{described}");
    assert_eq!(described["summary"]["profile"], GRANT_PROFILE);
    let mut scopes: Vec<&str> = described["granted_scopes"]
        .as_array()
        .unwrap_or_else(|| panic!("granted scopes: {described}"))
        .iter()
        .map(|scope| scope.as_str().unwrap())
        .collect();
    scopes.sort_unstable();
    let mut expected = vec![GRANT_SCOPE, GRANT_EXTRA_SCOPE];
    expected.sort_unstable();
    assert_eq!(scopes, expected);
    let expires = described["credential_expires_at_ms"]
        .as_u64()
        .unwrap_or_else(|| panic!("credential expiry: {described}"));
    let now = connectors_sdk::now_ms();
    // The fixture grants 3600 s.
    assert!(
        expires > now + 3_000_000 && expires <= now + 3_600_000,
        "{expires}"
    );
    assert_eq!(described["external_identity"]["subject"], "4242");
    assert!(!carries_grant_material(described.to_string().as_bytes()));
}

/// A refusal whose code is `code`, without any token on either stream.
#[track_caller]
fn refused(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    assert!(!carries_grant_material(&output.stderr));
    let text = String::from_utf8_lossy(&output.stderr);
    // A line the grant wrote ahead of the CLI's own refusal stays a line.
    let last = text.lines().last().unwrap_or_default();
    let value: Value = serde_json::from_str(last).unwrap_or_else(|_| panic!("{text}"));
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

/// A free loopback port for a fixed-port redirect.
fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_connect_with_loopback_fixed_port() {
    let port = free_port();
    let journey = GrantJourney::new(Behaviour::Consent, Redirect::Fixed(port));
    let connected = success(journey.connect(&[]))["connection"].clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(connected["summary"]["state"], "ready");
    assert_eq!(journey.server.issued(), 1, "one code was exchanged");
    // Consent went to the declared fixed port, with PKCE S256.
    let consent: Vec<_> = journey
        .server
        .requests()
        .into_iter()
        .filter(|seen| seen.route == "/authorize")
        .collect();
    assert_eq!(consent.len(), 1);
    let query = crate::oauth2_refresh::decode_form(consent[0].query.as_bytes());
    assert_eq!(
        crate::oauth2_refresh::field(&query, "redirect_uri"),
        Some(format!("http://127.0.0.1:{port}/callback").as_str())
    );
    assert_eq!(
        crate::oauth2_refresh::field(&query, "code_challenge_method"),
        Some("S256")
    );
    // The owner validated the stored response with its access token.
    assert_eq!(
        journey.server.api_authorizations(),
        vec![Some(format!("Bearer {GRANT_ACCESS}1"))]
    );
    assert_granted(&journey.describe(&reference), &reference);
    let facts = journey.cli.recorded();
    journey.assert_declared_servers(&facts);
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 1);
    completed(&acquisitions[0]);
    journey.assert_no_material_at_rest();
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_connect_with_pasted_code() {
    let journey = GrantJourney::new(Behaviour::Consent, Redirect::Pasted);
    let connected = success(journey.connect(&[]))["connection"].clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(journey.server.issued(), 1);
    let routes = journey.server.routes();
    assert!(routes.contains(&"GET /callback".to_owned()), "{routes:?}");
    assert_granted(&journey.describe(&reference), &reference);
    let facts = journey.cli.recorded();
    journey.assert_declared_servers(&facts);
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 1);
    completed(&acquisitions[0]);
    journey.assert_no_material_at_rest();
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_connect_with_device_grant() {
    let journey = GrantJourney::new(Behaviour::Consent, Redirect::AnyPort);
    let connected = success(journey.connect(&["--device"]))["connection"].clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(journey.server.issued(), 1);
    let routes = journey.server.routes();
    assert!(
        routes.contains(&"POST /device/code".to_owned()),
        "{routes:?}"
    );
    assert!(!routes.contains(&"GET /authorize".to_owned()), "{routes:?}");
    assert_granted(&journey.describe(&reference), &reference);
    let facts = journey.cli.recorded();
    journey.assert_declared_servers(&facts);
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 1);
    completed(&acquisitions[0]);
    journey.assert_no_material_at_rest();
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_reconnect_replaces_the_entry_on_the_same_connection() {
    let journey = GrantJourney::new(Behaviour::Consent, Redirect::AnyPort);
    let connected = success(journey.connect(&[]))["connection"].clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    journey.read(&reference);
    let before = journey.server.api_authorizations();
    assert_eq!(
        before.last().unwrap(),
        &Some(format!("Bearer {GRANT_ACCESS}1"))
    );

    let reconnected = success(journey.reconnect(&reference, &revision, &[]))["connection"].clone();
    assert_eq!(reconnected["summary"]["connection"], reference.as_str());
    // A same-principal renewal keeps the connection and its public revision
    // (contracts/cli/v1alpha1/semantics.md §5, `connections reconnect`).
    assert_eq!(reconnected["summary"]["revision"], revision.as_str());
    assert_eq!(journey.server.issued(), 2, "the grant ran again");
    assert_granted(&journey.describe(&reference), &reference);
    // A new owner and child hold no cached state: the read uses the new entry.
    journey.cli.shutdown();
    journey.read(&reference);
    let after = journey.server.api_authorizations();
    assert_eq!(
        after.last().unwrap(),
        &Some(format!("Bearer {GRANT_ACCESS}2"))
    );

    let facts = journey.cli.recorded();
    journey.assert_declared_servers(&facts);
    let connections: Vec<_> = facts
        .subjects(CONNECTION)
        .into_iter()
        .filter(|row| row.fields["instance_id"] == "fixture-grant")
        .collect();
    assert_eq!(connections.len(), 1, "the same connection");
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 2);
    completed(&acquisitions[0]);
    completed(&acquisitions[1]);
    journey.assert_no_material_at_rest();
}

/// A redirect whose `state` the flow never sent decides nothing: nothing is
/// exchanged, nothing stored, and the acquisition failed.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_state_mismatch_stores_nothing() {
    let journey = GrantJourney::new(Behaviour::ForgedState, Redirect::AnyPort);
    let refusal = refused(journey.connect(&[]), "protected_entry_unavailable");
    assert!(refusal["acquisition"].is_string(), "{refusal}");
    assert_eq!(journey.server.issued(), 0, "no code was exchanged");
    assert!(journey.server.api_authorizations().is_empty());
    let facts = journey.settled();
    journey.assert_declared_servers(&facts);
    assert!(facts.subjects(CUSTODY).is_empty(), "nothing stored");
    nothing_published(&facts);
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 1);
    failed(&acquisitions[0]);
    journey.assert_no_material_at_rest();
}

/// A device code the server answers `expired_token` ends the grant.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_device_expired_stores_nothing() {
    let journey = GrantJourney::new(Behaviour::DeviceExpired, Redirect::AnyPort);
    let refusal = refused(
        journey.connect(&["--device"]),
        "protected_entry_unavailable",
    );
    assert!(refusal["acquisition"].is_string(), "{refusal}");
    assert_eq!(journey.server.issued(), 0);
    let routes = journey.server.routes();
    assert!(
        routes.contains(&"POST /device/code".to_owned()),
        "{routes:?}"
    );
    let facts = journey.settled();
    journey.assert_declared_servers(&facts);
    assert!(facts.subjects(CUSTODY).is_empty(), "nothing stored");
    nothing_published(&facts);
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 1);
    failed(&acquisitions[0]);
    journey.assert_no_material_at_rest();
}

/// A grant without the declaration's required scope is refused as
/// `not_granted` (the CLI's code for an insufficient scope) before it reaches
/// the owner: nothing is validated with it and nothing stored.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth2_grant_missing_required_scope_stores_nothing() {
    let journey = GrantJourney::new(Behaviour::NarrowScope, Redirect::AnyPort);
    let refusal = refused(journey.connect(&[]), "not_granted");
    assert!(refusal["acquisition"].is_string(), "{refusal}");
    assert_eq!(journey.server.issued(), 1, "the code was exchanged");
    assert!(
        journey.server.api_authorizations().is_empty(),
        "the narrow token was never used"
    );
    let facts = journey.settled();
    journey.assert_declared_servers(&facts);
    assert!(facts.subjects(CUSTODY).is_empty(), "nothing stored");
    nothing_published(&facts);
    let acquisitions = journey.acquisitions(&facts);
    assert_eq!(acquisitions.len(), 1);
    failed(&acquisitions[0]);
    journey.assert_no_material_at_rest();
}

#[path = "token_journey.rs"]
mod token_journey;
