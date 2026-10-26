//! The shipped `claude-subscription` profiles through the production CLI, owner, catalog child
//! and disposable Secret Service (story:claude-subscription-connector). The instance is the
//! shipped profile with its configured client id, its endpoints moved onto the fixture of
//! [`crate::claude_subscription`]. `connections connect` runs the grant in the CLI process: the
//! debug-build follow hook (`CONNECTORS_TEST_OAUTH_FOLLOW`, the fixture root the only trusted
//! one) consents where a person would and follows the loopback or pasted-code redirect.
//! `connections token` refreshes an access token inside the declared lead. No output stream and
//! no file at rest carries a token or code.
use super::*;
use crate::claude_subscription::{
    CLAUDE_ACCESS, CLAUDE_ACCOUNT, CLAUDE_CLIENT_ID, CLAUDE_LOOPBACK, CLAUDE_PASTED,
    CLAUDE_REFRESH, CLAUDE_SCOPES, ClaudeServer, EXCHANGE_FIELDS, Lifetime, REFRESH_FIELDS,
    carries_claude_material, claude_config,
};

/// The pinned instance the guide configures (website/docs/connectors/claude-subscription.md).
const CONNECTOR: &str = "claude-subscription";
const INSTANCE: &str = "mine";
const TABLE: &str = "connectors.claude-subscription.instances.mine";

struct ClaudeJourney {
    // Dropped in this order: the owner stops before custody and the fixture.
    cli: Cli,
    _custody: Custody,
    server: ClaudeServer,
    profile: &'static str,
}

impl ClaudeJourney {
    fn new(profile: &'static str, lifetime: Lifetime) -> Self {
        let server = ClaudeServer::new(lifetime);
        server.write_config(&claude_config(
            server.port,
            &server.ca,
            profile,
            Some(CLAUDE_CLIENT_ID),
        ));
        let custody = Custody::new(server.root.path());
        let cli = Cli::new(server.root.path());
        success(cli.run(&["setup", "init"]));
        success(cli.run(&["setup", "check"]));
        let adapter = server.selection();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let configuration = format!(
            "format='connectors-local/5'\nowner_uid={}\nsecret_service_socket={}\n\
             [connectors.claude-subscription]\nenabled=true\n\
             [{TABLE}]\nalias='claude'\ninstance_id={}\nadapter_id='catalog'\n\
             configuration_revision={}\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n\
             startup='on-demand'\nrestart='never'\n\
             [{TABLE}.permissions]\nprofiles=[{}]\noperations=['account.profile']\n\
             [{TABLE}.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&adapter.instance_id),
            q(&adapter.configuration_revision),
            q(profile),
            q(adapter.executable.path.to_str().unwrap()),
            q(&adapter.executable.sha256),
            serde_json::to_string(&adapter.executable.args).unwrap()
        );
        private(&cli.paths.config, configuration.as_bytes());
        Self {
            cli,
            _custody: custody,
            server,
            profile,
        }
    }

    /// Runs `args` with the follow hook and checks that no token or code reaches either stream.
    fn follow(&self, args: &[&str]) -> Output {
        let output = self.cli.run_following(args, &self.server.ca);
        assert!(
            !carries_claude_material(&output.stdout),
            "material on stdout"
        );
        assert!(
            !carries_claude_material(&output.stderr),
            "material on stderr"
        );
        output
    }

    /// Connects, and answers the new connection's reference.
    fn connect(&self) -> String {
        let connected = success(self.follow(&[
            "connections",
            "connect",
            "--connector",
            CONNECTOR,
            "--instance",
            INSTANCE,
            "--profile",
            self.profile,
        ]))["connection"]
            .clone();
        assert_eq!(connected["summary"]["state"], "ready", "{connected}");
        connected["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn describe(&self, reference: &str) -> Value {
        let output = self.cli.run(&[
            "connections",
            "describe",
            "--connector",
            CONNECTOR,
            "--instance",
            INSTANCE,
            "--connection",
            reference,
        ]);
        assert!(!carries_claude_material(&output.stdout));
        success(output)["connection"].clone()
    }

    /// The guide's first read: `account.profile`, its schema and revision from `describe`.
    fn read_profile(&self, reference: &str) -> Value {
        let selector = ["--connector", CONNECTOR, "--instance", INSTANCE];
        let mut describe = vec!["operations", "describe"];
        describe.extend(selector);
        describe.extend(["--operation", "account.profile"]);
        let description = success(self.cli.run(&describe));
        let mut invoke = vec!["operations", "invoke"];
        invoke.extend(selector);
        invoke.extend([
            "--connection",
            reference,
            "--operation",
            "account.profile",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            "{}",
        ]);
        let output = self.cli.run(&invoke);
        assert!(!carries_claude_material(&output.stdout));
        assert!(!carries_claude_material(&output.stderr));
        success(output)
    }

    /// `connections token` of `reference`; the owner it may start trusts the fixture root.
    fn token(&self, reference: &str, extra: &[&str]) -> Output {
        let mut args = vec![
            "connections",
            "token",
            "--connector",
            CONNECTOR,
            "--instance",
            INSTANCE,
            "--connection",
            reference,
        ];
        args.extend_from_slice(extra);
        self.cli.run_following(&args, &self.server.ca)
    }

    /// The configuration, the owner's state and the provider configuration carry no token at
    /// rest: the entry lives in custody only.
    fn assert_no_material_at_rest(&self) {
        let mut written = vec![self.cli.paths.config.clone(), self.server.config.clone()];
        files_under(&self.cli.paths.state, &mut written);
        for path in &written {
            let bytes = fs::read(path).unwrap_or_default();
            assert!(!carries_claude_material(&bytes), "{}", path.display());
        }
    }

    /// The one consent: the declared extra, scopes and client, with PKCE S256. Answers its
    /// `state` and `redirect_uri`.
    fn consented(&self) -> (String, String) {
        let consent: Vec<_> = self
            .server
            .requests()
            .into_iter()
            .filter(|seen| seen.route == "/cai/oauth/authorize")
            .collect();
        assert_eq!(consent.len(), 1);
        let query = crate::oauth2_refresh::decode_form(consent[0].query.as_bytes());
        let get = |name: &str| crate::oauth2_refresh::field(&query, name).map(str::to_owned);
        assert_eq!(get("code").as_deref(), Some("true"));
        assert_eq!(get("client_id").as_deref(), Some(CLAUDE_CLIENT_ID));
        assert_eq!(get("scope"), Some(CLAUDE_SCOPES.join(" ")));
        assert_eq!(get("code_challenge_method").as_deref(), Some("S256"));
        (get("state").unwrap(), get("redirect_uri").unwrap())
    }

    /// The one code exchange: JSON, exactly the story's fields, the flow's own `state` and the
    /// consent's redirect.
    fn assert_exchanged(&self, state: &str, redirect_uri: &str) {
        let requests = self.server.token_requests();
        let exchanges: Vec<_> = requests
            .iter()
            .filter(|request| request.fields["grant_type"] == "authorization_code")
            .collect();
        assert_eq!(exchanges.len(), 1);
        let exchange = exchanges[0];
        assert_eq!(exchange.content_type.as_deref(), Some("application/json"));
        assert_eq!(exchange.keys(), EXCHANGE_FIELDS);
        assert_eq!(exchange.fields["state"], state);
        assert_eq!(exchange.fields["redirect_uri"], redirect_uri);
        assert_eq!(exchange.fields["client_id"], CLAUDE_CLIENT_ID);
    }
}

/// The connection is ready, holds both scopes, and is the fixture's account.
#[track_caller]
fn assert_granted(described: &Value, profile: &str) {
    assert_eq!(described["summary"]["state"], "ready", "{described}");
    assert_eq!(described["summary"]["profile"], profile);
    let mut scopes: Vec<&str> = described["granted_scopes"]
        .as_array()
        .unwrap_or_else(|| panic!("granted scopes: {described}"))
        .iter()
        .map(|scope| scope.as_str().unwrap())
        .collect();
    scopes.sort_unstable();
    let mut expected = CLAUDE_SCOPES.to_vec();
    expected.sort_unstable();
    assert_eq!(scopes, expected);
    assert_eq!(described["external_identity"]["subject"], CLAUDE_ACCOUNT);
    assert!(described["credential_expires_at_ms"].as_u64().is_some());
    assert!(!carries_claude_material(described.to_string().as_bytes()));
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn claude_connect_loopback_journey() {
    let journey = ClaudeJourney::new(CLAUDE_LOOPBACK, Lifetime::Hour);
    let reference = journey.connect();
    let (state, redirect_uri) = journey.consented();
    // The declared template's host and path, at the port the receiver bound.
    let port = redirect_uri
        .strip_prefix("http://localhost:")
        .and_then(|rest| rest.strip_suffix("/callback"))
        .unwrap_or_else(|| panic!("loopback redirect {redirect_uri}"));
    assert!(port.parse::<u16>().is_ok(), "{redirect_uri}");
    journey.assert_exchanged(&state, &redirect_uri);
    // The owner proved the stored response by reading the profile with its access token.
    assert_eq!(
        journey.server.profile_authorizations(),
        vec![Some(format!("Bearer {CLAUDE_ACCESS}1"))]
    );
    assert_granted(&journey.describe(&reference), CLAUDE_LOOPBACK);
    let routes = journey.server.routes();
    assert!(
        !routes.contains(&"GET /oauth/code/callback".to_owned()),
        "{routes:?}"
    );
    // The first read sends the same access token to the profile and answers the account.
    let read = journey.read_profile(&reference);
    assert!(read.to_string().contains(CLAUDE_ACCOUNT), "{read}");
    assert_eq!(
        journey.server.profile_authorizations(),
        vec![Some(format!("Bearer {CLAUDE_ACCESS}1")); 2]
    );
    journey.assert_no_material_at_rest();
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn claude_connect_pasted_code_journey() {
    let journey = ClaudeJourney::new(CLAUDE_PASTED, Lifetime::Hour);
    let reference = journey.connect();
    let (state, redirect_uri) = journey.consented();
    assert_eq!(
        redirect_uri,
        format!(
            "https://localhost:{}/oauth/code/callback",
            journey.server.port
        )
    );
    let routes = journey.server.routes();
    assert!(
        routes.contains(&"GET /oauth/code/callback".to_owned()),
        "{routes:?}"
    );
    journey.assert_exchanged(&state, &redirect_uri);
    assert_eq!(
        journey.server.profile_authorizations(),
        vec![Some(format!("Bearer {CLAUDE_ACCESS}1"))]
    );
    assert_granted(&journey.describe(&reference), CLAUDE_PASTED);
    journey.assert_no_material_at_rest();
}

/// Each access token lives 120 s, inside the declared 300 s lead, so each `connections token`
/// refreshes: a JSON body of exactly the story's fields with `scope`, answered without a refresh
/// token, so the second refresh sends the first refresh token again.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn claude_token_refreshes_with_json() {
    let journey = ClaudeJourney::new(CLAUDE_LOOPBACK, Lifetime::Short);
    let reference = journey.connect();
    for round in 1..=2 {
        let output = journey.token(&reference, &[]);
        assert!(!carries_claude_material(&output.stdout), "token on stdout");
        assert!(!carries_claude_material(&output.stderr), "token on stderr");
        let result = success(output);
        assert_eq!(result["refreshed"], true, "round {round}: {result}");
    }
    let refreshes: Vec<_> = journey
        .server
        .token_requests()
        .into_iter()
        .filter(|request| {
            request.fields.get("grant_type").map(String::as_str) != Some("authorization_code")
        })
        .collect();
    assert_eq!(refreshes.len(), 2, "one refresh per token");
    for refresh in &refreshes {
        assert_eq!(refresh.content_type.as_deref(), Some("application/json"));
        assert_eq!(refresh.keys(), REFRESH_FIELDS);
        assert_eq!(refresh.fields["grant_type"], "refresh_token");
        assert_eq!(refresh.fields["client_id"], CLAUDE_CLIENT_ID);
        assert_eq!(refresh.fields["scope"], CLAUDE_SCOPES.join(" "));
        // The answers carried no refresh token: the one the grant issued stays.
        assert!(refresh.fields["refresh_token"] == format!("{CLAUDE_REFRESH}1"));
    }
    let revealed = journey.token(&reference, &["--reveal"]);
    assert!(revealed.status.success());
    assert_eq!(revealed.stdout, format!("{CLAUDE_ACCESS}4\n").into_bytes());
    assert_eq!(journey.describe(&reference)["summary"]["state"], "ready");
    journey.assert_no_material_at_rest();
}
