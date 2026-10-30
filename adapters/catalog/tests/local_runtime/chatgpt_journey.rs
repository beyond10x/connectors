//! The `chatgpt` connector through the production CLI, owner, catalog child
//! and disposable Secret Service (story:chatgpt-subscription-connector), with
//! the fake authorization server of [`crate::chatgpt`] as the only trusted
//! root. `connections connect` signs in by the browser (the debug-build follow
//! hook consents where a person would) or by the ChatGPT device sign-in, the
//! dialect composition registers for `chatgpt.subscription`; the owner
//! refreshes with JSON and keeps each rotated refresh token; and every
//! journey ends with `connections token --reveal` printing the access token
//! the fake server issued last.
use super::*;
use crate::chatgpt::{
    CHATGPT_ACCESS, CHATGPT_ACCOUNT, CHATGPT_CLIENT_ID, CHATGPT_INSTANCE, CHATGPT_PROFILE,
    CHATGPT_REFRESH, CHATGPT_SCOPE, ChatgptServer, carries_chatgpt_material,
};
use crate::oauth2_refresh::{decode_form, field};

/// The alias of the fixture's pinned instance.
const ALIAS: &str = "chatgpt";

struct ChatgptJourney {
    // Dropped in this order: the owner stops before custody and the fixture.
    cli: Cli,
    _custody: Custody,
    server: ChatgptServer,
}

impl ChatgptJourney {
    /// A fixture whose access tokens live `lifetime` seconds, and a CLI with
    /// one pinned `chatgpt` instance: its client id is its configuration's.
    fn new(lifetime: u64, redirect_port: u16) -> Self {
        let server = ChatgptServer::new(lifetime, redirect_port);
        let custody = Custody::new(server.root.path());
        let cli = Cli::new(server.root.path());
        success(cli.run(&["setup", "init"]));
        success(cli.run(&["setup", "check"]));
        let adapter = server.selection();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let configuration = format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.{ALIAS}]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.{ALIAS}.permissions]\nprofiles=[{}]\noperations=['models.list']\n[adapters.{ALIAS}.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&adapter.instance_id),
            q(&adapter.configuration_revision),
            q(CHATGPT_PROFILE),
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

    /// `connections connect` of the profile with the follow hook; no token,
    /// code or verifier reaches either output stream.
    fn connect(&self, extra: &[&str]) -> Output {
        let mut args = vec![
            "connections",
            "connect",
            "--adapter",
            ALIAS,
            "--profile",
            CHATGPT_PROFILE,
        ];
        args.extend_from_slice(extra);
        let output = self.cli.run_following(&args, &self.server.ca);
        assert!(
            !carries_chatgpt_material(&output.stdout),
            "material on stdout"
        );
        assert!(
            !carries_chatgpt_material(&output.stderr),
            "material on stderr"
        );
        output
    }

    fn describe(&self, reference: &str) -> Value {
        let output = self.cli.run(&[
            "connections",
            "describe",
            "--adapter",
            ALIAS,
            "--connection",
            reference,
        ]);
        assert!(!carries_chatgpt_material(&output.stdout));
        success(output)["connection"].clone()
    }

    /// `connections token` of `reference`; the owner it may start trusts the
    /// fixture root, so its refresh reaches the fixture.
    fn token(&self, reference: &str, extra: &[&str]) -> Output {
        let mut args = vec![
            "connections",
            "token",
            "--adapter",
            ALIAS,
            "--connection",
            reference,
        ];
        args.extend_from_slice(extra);
        self.cli.run_following(&args, &self.server.ca)
    }

    /// `connections token --reveal` prints exactly the access token the
    /// fixture issued last, and one newline.
    #[track_caller]
    fn reveals_the_current_token(&self, reference: &str) {
        let revealed = self.token(reference, &["--reveal"]);
        assert!(
            revealed.status.success(),
            "{}",
            String::from_utf8_lossy(&revealed.stderr)
        );
        assert!(revealed.stderr.is_empty());
        assert_eq!(
            revealed.stdout,
            format!("{CHATGPT_ACCESS}{}\n", self.server.issued()).into_bytes(),
            "stdout is the fake server's current token and a newline"
        );
    }

    /// The connection is ready, under the profile, and named by the ChatGPT
    /// account id; its description shows no token.
    #[track_caller]
    fn assert_connected(&self, reference: &str) {
        let described = self.describe(reference);
        assert_eq!(described["summary"]["connection"], reference);
        assert_eq!(described["summary"]["state"], "ready", "{described}");
        assert_eq!(described["summary"]["profile"], CHATGPT_PROFILE);
        assert_eq!(
            described["external_identity"]["subject"], CHATGPT_ACCOUNT,
            "{described}"
        );
        assert_eq!(described["external_identity"]["kind"], "chatgpt.account");
        assert!(!carries_chatgpt_material(described.to_string().as_bytes()));
    }

    /// `models.list` through `reference`: the result the fixture answered.
    fn read(&self, reference: &str) -> Value {
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            ALIAS,
            "--operation",
            "models.list",
        ]));
        let output = self.cli.run(&[
            "invoke",
            &format!("chatgpt/{ALIAS}"),
            "models.list",
            "--connection",
            reference,
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            r#"{"client_version":"0.0.0-fixture"}"#,
        ]);
        assert!(!carries_chatgpt_material(&output.stdout));
        assert!(!carries_chatgpt_material(&output.stderr));
        success(output)
    }

    /// The `Authorization` header of every read, in order.
    fn read_authorizations(&self) -> Vec<Option<String>> {
        self.server
            .requests()
            .into_iter()
            .filter(|seen| seen.route == "/backend-api/codex/models")
            .map(|seen| seen.authorization)
            .collect()
    }

    /// The configuration, the owner's state and the provider configuration
    /// carry no token at rest: the entry lives in custody only.
    fn assert_no_material_at_rest(&self) {
        let mut written = vec![self.cli.paths.config.clone(), self.server.config.clone()];
        files_under(&self.cli.paths.state, &mut written);
        for path in &written {
            let bytes = fs::read(path).unwrap_or_default();
            assert!(!carries_chatgpt_material(&bytes), "{}", path.display());
        }
    }

    /// The bodies of the token endpoint's requests, in order, each with its
    /// content type.
    fn token_requests(&self) -> Vec<(Option<String>, Vec<u8>)> {
        self.server
            .requests()
            .into_iter()
            .filter(|seen| seen.method == "POST" && seen.route == "/oauth/token")
            .map(|seen| (seen.content_type, seen.body))
            .collect()
    }
}

/// The connection a successful connect answered.
#[track_caller]
fn connection_of(output: Output) -> String {
    let connected = success(output)["connection"].clone();
    assert_eq!(connected["summary"]["state"], "ready", "{connected}");
    connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// A free loopback port for the fixed-port redirect.
fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// The browser sign-in: the authorize request carries the configured client,
/// the Codex CLI's scope string and extras, S256 and the fixed loopback
/// redirect; the code is exchanged with a form; the connection is named by
/// the account id; and `--reveal` prints the token the exchange issued.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn chatgpt_connect_browser_journey() {
    let port = free_port();
    let journey = ChatgptJourney::new(3600, port);
    let reference = connection_of(journey.connect(&[]));
    assert_eq!(journey.server.issued(), 1, "one code was exchanged");
    let consent: Vec<_> = journey
        .server
        .requests()
        .into_iter()
        .filter(|seen| seen.route == "/oauth/authorize")
        .collect();
    assert_eq!(consent.len(), 1);
    let query = &consent[0].query;
    for (name, value) in [
        ("response_type", "code"),
        ("client_id", CHATGPT_CLIENT_ID),
        ("scope", CHATGPT_SCOPE),
        ("code_challenge_method", "S256"),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("originator", "codex_cli_rs"),
    ] {
        assert_eq!(field(query, name), Some(value), "{name}");
    }
    let redirect = format!("http://127.0.0.1:{port}/auth/callback");
    assert_eq!(field(query, "redirect_uri"), Some(redirect.as_str()));
    let exchanges = journey.token_requests();
    assert_eq!(exchanges.len(), 1);
    assert_eq!(
        exchanges[0].0.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    let form = decode_form(&exchanges[0].1);
    let mut names: Vec<&str> = form.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "client_id",
            "code",
            "code_verifier",
            "grant_type",
            "redirect_uri"
        ]
    );
    journey.assert_connected(&reference);
    // The connector's one read carries the granted access token.
    let read = journey.read(&reference);
    assert!(read.to_string().contains("fixture-model"), "{read}");
    let query: Vec<_> = journey
        .server
        .requests()
        .into_iter()
        .filter(|seen| seen.route == "/backend-api/codex/models")
        .map(|seen| seen.query)
        .collect();
    assert_eq!(
        query,
        [vec![(
            "client_version".to_owned(),
            "0.0.0-fixture".to_owned()
        )]]
    );
    assert_eq!(
        journey.read_authorizations(),
        [Some(format!("Bearer {CHATGPT_ACCESS}1"))]
    );
    journey.reveals_the_current_token(&reference);
    assert_eq!(journey.server.issued(), 1, "no refresh was needed");
    journey.assert_no_material_at_rest();
}

/// The device sign-in, which is not RFC 8628: `--device` runs the dialect
/// registered for the profile. It asks a user code with the configured
/// client, shows it, polls through one pending answer, and the code and
/// verifier it receives are exchanged with a form for the device callback.
/// No browser authorization is asked.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn chatgpt_connect_device_journey() {
    let journey = ChatgptJourney::new(3600, free_port());
    let output = journey.connect(&["--device"]);
    // Without a terminal the user code and the page are one stderr line.
    let page = format!("https://localhost:{}/codex/device", journey.server.port);
    let prompt = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        prompt.is_empty() || prompt == format!("connectors: device-code FIXT-0001 {page}\n"),
        "{prompt}"
    );
    let output = Output {
        stderr: Vec::new(),
        ..output
    };
    let reference = connection_of(output);
    assert_eq!(
        journey.server.routes(),
        [
            "POST /api/accounts/deviceauth/usercode",
            "POST /api/accounts/deviceauth/token",
            "POST /api/accounts/deviceauth/token",
            "POST /oauth/token",
        ]
    );
    let requests = journey.server.requests();
    let start: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(start, json!({"client_id": CHATGPT_CLIENT_ID}));
    assert_eq!(
        requests[0].content_type.as_deref(),
        Some("application/json")
    );
    let exchanges = journey.token_requests();
    assert_eq!(
        exchanges[0].0.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    let form = decode_form(&exchanges[0].1);
    let callback = format!(
        "https://localhost:{}/deviceauth/callback",
        journey.server.port
    );
    assert_eq!(field(&form, "redirect_uri"), Some(callback.as_str()));
    assert_eq!(field(&form, "grant_type"), Some("authorization_code"));
    assert_eq!(journey.server.issued(), 1);
    journey.assert_connected(&reference);
    journey.reveals_the_current_token(&reference);
    journey.assert_no_material_at_rest();
}

/// Every access token lives 120 s, inside the declared 300 s lead, so each
/// `connections token` refreshes. Each refresh is the JSON
/// `{grant_type, client_id, refresh_token}` with no scope, presenting the
/// refresh token the previous answer rotated in; the spent one is never sent
/// again. The connection keeps its account identity, and `--reveal` prints
/// the token the last refresh issued.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn chatgpt_token_refreshes_with_json_and_rotates() {
    let journey = ChatgptJourney::new(120, free_port());
    let reference = connection_of(journey.connect(&[]));
    assert_eq!(journey.server.issued(), 1);
    let first = success(journey.token(&reference, &[]));
    assert_eq!(first["refreshed"], true, "{first}");
    assert_eq!(journey.server.issued(), 2);
    journey.reveals_the_current_token(&reference);
    assert_eq!(journey.server.issued(), 3, "the reveal refreshed again");
    assert_eq!(
        journey.server.spent(),
        [format!("{CHATGPT_REFRESH}1"), format!("{CHATGPT_REFRESH}2")],
        "each refresh presented the token the previous one rotated in"
    );
    let requests = journey.token_requests();
    assert_eq!(requests.len(), 3, "one exchange and two refreshes");
    for (index, (content_type, body)) in requests[1..].iter().enumerate() {
        assert_eq!(content_type.as_deref(), Some("application/json"));
        let body: Value = serde_json::from_slice(body).unwrap();
        assert!(
            body == json!({
                "grant_type": "refresh_token",
                "client_id": CHATGPT_CLIENT_ID,
                "refresh_token": format!("{CHATGPT_REFRESH}{}", index + 1),
            }),
            "refresh {index} is the JSON the Codex CLI sends"
        );
    }
    journey.assert_connected(&reference);
    assert_eq!(journey.server.issued(), 3);
    let facts = journey.cli.recorded();
    assert!(
        facts
            .subjects("connectors.auth_bindings.Connection")
            .into_iter()
            .filter(|row| row.fields["instance_id"] == CHATGPT_INSTANCE)
            .count()
            == 1,
        "one connection throughout"
    );
    journey.assert_no_material_at_rest();
}
