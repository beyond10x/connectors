//! `connections token` and the library's access token through the production
//! CLI, owner, catalog child and disposable Secret Service
//! (story:connections-token-issues-an-access-token). The fixture of
//! [`super`] is a rotating authorization server: a refresh spends the
//! refresh token it was sent and answers a new access and refresh token.
//! Without `--reveal` neither output stream carries any token; with it,
//! standard output is the access token and one newline. Each issue is one
//! recorded `connectors.oauth2.AccessTokenIssue` naming its delivery, and a
//! refusal records none.
use super::*;

const ISSUE: &str = "connectors.oauth2.AccessTokenIssue";

impl GrantJourney {
    /// A connected `grant` instance, and its connection.
    fn connected(behaviour: Behaviour) -> (Self, String) {
        let journey = Self::new(behaviour, Redirect::AnyPort);
        let connected = success(journey.connect(&[]))["connection"].clone();
        let reference = connected["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();
        (journey, reference)
    }

    /// `connections token` of `reference` with `--output json`. The owner it
    /// may start trusts the fixture root, as the connect's owner does, so its
    /// refresh reaches the fixture.
    fn token(&self, reference: &str, extra: &[&str]) -> Output {
        let mut args = vec![
            "connections",
            "token",
            "--adapter",
            "grant",
            "--connection",
            reference,
        ];
        args.extend_from_slice(extra);
        self.cli.run_following(&args, &self.server.ca)
    }

    /// `connections token` of `reference` with the default text output.
    fn token_text(&self, reference: &str, extra: &[&str]) -> Output {
        let mut command = Command::new(&self.cli.binary);
        command
            .env_clear()
            .env(
                "CONNECTORS_TEST_OWNER_PARENT",
                std::process::id().to_string(),
            )
            .env("CONNECTORS_TEST_OAUTH_FOLLOW", &self.server.ca)
            .arg("--config")
            .arg(&self.cli.paths.config)
            .arg("--state-dir")
            .arg(&self.cli.paths.state)
            .args([
                "connections",
                "token",
                "--adapter",
                "grant",
                "--connection",
                reference,
            ])
            .args(extra)
            .stdin(Stdio::null());
        command.output().unwrap()
    }

    /// The token requests the fixture answered, the code exchange included.
    fn token_requests(&self) -> usize {
        self.server
            .routes()
            .iter()
            .filter(|route| *route == "POST /token")
            .count()
    }

    /// Every recorded issue, each with its one event.
    fn issues(&self) -> Vec<Value> {
        let facts = self.cli.recorded();
        facts
            .subjects(ISSUE)
            .into_iter()
            .map(|row| {
                let transitions = facts.transitions(row);
                assert_eq!(transitions.len(), 1, "issued once: {transitions:?}");
                assert_eq!(transitions[0].from, None);
                assert_eq!(transitions[0].to, "Issued");
                assert_eq!(
                    transitions[0].events,
                    vec!["connectors.oauth2.AccessTokenIssued".to_owned()]
                );
                assert!(!carries_grant_material(
                    serde_json::to_string(&row.fields).unwrap().as_bytes()
                ));
                let mut fields = serde_json::to_value(&row.fields).unwrap();
                fields["issue_ref"] = json!(row.id.strip_prefix("s:").unwrap_or(&row.id));
                fields
            })
            .collect()
    }
}

/// The deliveries of `issues`, sorted.
fn deliveries(issues: &[Value]) -> Vec<String> {
    let mut deliveries: Vec<String> = issues
        .iter()
        .map(|issue| issue["delivery"].as_str().unwrap().to_owned())
        .collect();
    deliveries.sort();
    deliveries
}

/// A successful `connections token` result that names no token.
#[track_caller]
fn issued(output: Output) -> Value {
    assert!(!carries_grant_material(&output.stdout), "token on stdout");
    assert!(!carries_grant_material(&output.stderr), "token on stderr");
    let result = success(output);
    assert!(result["expires_at"].is_string(), "{result}");
    assert!(result["issue"].is_string(), "{result}");
    assert!(result["refreshed"].is_boolean(), "{result}");
    result
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn token_without_reveal_prints_no_token() {
    let (journey, reference) = GrantJourney::connected(Behaviour::Consent);
    let result = issued(journey.token(&reference, &[]));
    assert_eq!(result["connection"], reference);
    assert_eq!(result["refreshed"], false);
    let mut scopes: Vec<&str> = result["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|scope| scope.as_str().unwrap())
        .collect();
    scopes.sort_unstable();
    let mut expected = vec![GRANT_SCOPE, GRANT_EXTRA_SCOPE];
    expected.sort_unstable();
    assert_eq!(scopes, expected);
    let text = journey.token_text(&reference, &[]);
    assert!(text.status.success());
    assert!(!carries_grant_material(&text.stdout), "token on stdout");
    assert!(!carries_grant_material(&text.stderr), "token on stderr");
    let shown = String::from_utf8_lossy(&text.stdout);
    assert!(
        shown.contains("expires_at") && shown.contains("issue"),
        "{shown}"
    );
    // No refresh was needed: only the grant's code exchange reached the server.
    assert_eq!(journey.token_requests(), 1);
    let issues = journey.issues();
    assert_eq!(deliveries(&issues), ["metadata_only", "metadata_only"]);
    for issue in &issues {
        assert_eq!(issue["connection_ref"], reference);
        assert!(issue.get("refresh_attempt_id").is_none_or(Value::is_null));
    }
    assert!(
        issues
            .iter()
            .any(|issue| issue_ref(issue) == result["issue"].as_str().unwrap()),
        "the result names its recorded issue"
    );
    journey.assert_no_material_at_rest();
}

/// The logical id of a recorded issue.
fn issue_ref(issue: &Value) -> &str {
    issue["issue_ref"].as_str().unwrap()
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn token_with_reveal_prints_only_the_token() {
    let (journey, reference) = GrantJourney::connected(Behaviour::Consent);
    let expected = format!("{GRANT_ACCESS}1\n").into_bytes();
    for (output, json) in [
        (journey.token(&reference, &["--reveal"]), true),
        (journey.token(&reference, &["--reveal", "true"]), true),
        (journey.token_text(&reference, &["--reveal"]), false),
    ] {
        assert!(output.status.success());
        assert_eq!(output.stdout, expected, "stdout is the token and a newline");
        assert!(!carries_grant_material(&output.stderr), "token on stderr");
        // Under text output the deprecated `--adapter` says so on stderr.
        if json {
            assert!(output.stderr.is_empty());
        }
    }
    assert_eq!(journey.token_requests(), 1);
    assert_eq!(
        deliveries(&journey.issues()),
        ["revealed", "revealed", "revealed"]
    );
    journey.assert_no_material_at_rest();
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn token_within_lead_time_refreshes_once() {
    // The grant's access token is valid for 120 s, inside the declared 300 s
    // lead; the refreshed one for 3600 s.
    let (journey, reference) = GrantJourney::connected(Behaviour::ShortLived);
    assert_eq!(journey.token_requests(), 1);
    let first = issued(journey.token(&reference, &[]));
    assert_eq!(first["refreshed"], true);
    assert_eq!(journey.token_requests(), 2, "one refresh request");
    assert_eq!(journey.server.refreshes(), 1);
    let second = issued(journey.token(&reference, &[]));
    assert_eq!(second["refreshed"], false);
    assert_eq!(second["expires_at"], first["expires_at"]);
    let revealed = journey.token(&reference, &["--reveal"]);
    assert!(revealed.status.success());
    assert_eq!(revealed.stdout, format!("{GRANT_ACCESS}2\n").into_bytes());
    assert_eq!(journey.token_requests(), 2, "no request after the refresh");
    let issues = journey.issues();
    assert_eq!(
        deliveries(&issues),
        ["metadata_only", "metadata_only", "revealed"]
    );
    let refreshed: Vec<_> = issues
        .iter()
        .filter(|issue| issue["refresh_attempt_id"].is_string())
        .collect();
    assert_eq!(
        refreshed.len(),
        1,
        "one issue names the refresh: {issues:?}"
    );
    assert_eq!(issue_ref(refreshed[0]), first["issue"].as_str().unwrap());
    let generation = &refreshed[0]["generation_id"];
    assert!(
        issues
            .iter()
            .all(|issue| &issue["generation_id"] == generation),
        "all three issued the refreshed generation"
    );
    journey.assert_no_material_at_rest();
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn token_on_a_stale_connection_refuses_with_repair_connection() {
    // The refresh the expiring token needs is answered `invalid_grant`.
    let (journey, reference) = GrantJourney::connected(Behaviour::RefreshRefused);
    let refusal = refused(journey.token(&reference, &["--reveal"]), "service_failure");
    assert_eq!(refusal["next_action"], "repair_connection", "{refusal}");
    assert_eq!(journey.token_requests(), 2);
    assert_eq!(journey.describe(&reference)["summary"]["state"], "stale");
    // Stale now: refused again without a request.
    let refusal = refused(journey.token(&reference, &[]), "service_failure");
    assert_eq!(refusal["next_action"], "repair_connection", "{refusal}");
    assert_eq!(journey.token_requests(), 2);
    assert!(journey.issues().is_empty(), "a refusal records nothing");
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn token_on_a_static_profile_is_forbidden() {
    let provider = Provider::basic("api");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &basic(BASIC_TOKEN).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        BASIC_PROFILE,
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    let reference = connected["summary"]["connection"].as_str().unwrap();
    for extra in [&[][..], &["--reveal"][..]] {
        let mut args = vec![
            "connections",
            "token",
            "--adapter",
            "gitlab",
            "--connection",
            reference,
        ];
        args.extend_from_slice(extra);
        let output = cli.run(&args);
        assert!(!carries_basic_material(&output.stdout));
        assert!(!carries_basic_material(&output.stderr));
        let refusal = super::super::refusal(output, "forbidden");
        assert_eq!(refusal["stage"], "admission", "{refusal}");
    }
    // The selected instance's one connection is selected without --connection.
    let refusal = super::super::refusal(
        cli.run(&["connections", "token", "--adapter", "gitlab"]),
        "forbidden",
    );
    assert_eq!(refusal["next_action"], "request_permission", "{refusal}");
    assert!(cli.recorded().subjects(ISSUE).is_empty());
}

#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn library_access_token_matches_the_cli() {
    let (journey, reference) = GrantJourney::connected(Behaviour::Consent);
    let revealed = journey.token(&reference, &["--reveal"]);
    assert!(revealed.status.success());
    let cli_token = String::from_utf8(revealed.stdout).unwrap();
    let token =
        connectors_host::local::access_token::access_token(&journey.cli.paths, "grant", &reference)
            .unwrap();
    assert_eq!(token.token(), Some(cli_token.trim_end()));
    assert_eq!(token.connection(), reference);
    assert!(!token.refreshed());
    let shown = format!("{token:?} {token:#?} {token}");
    assert!(!carries_grant_material(shown.as_bytes()), "{shown}");
    assert_eq!(journey.token_requests(), 1);
    let issues = journey.issues();
    assert_eq!(deliveries(&issues), ["library", "revealed"]);
    assert!(
        issues.iter().any(|issue| issue_ref(issue) == token.issue()),
        "the library's issue is recorded"
    );
}
