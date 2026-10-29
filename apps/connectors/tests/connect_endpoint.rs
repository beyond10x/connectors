//! story:connectors-do-short-invocation, decision D10: the endpoint is a
//! property of the instance, given once. `connections connect
//! <connector>/<instance> --endpoint <url>` writes `endpoint` into the
//! instance's table of the configuration file before any credential is asked
//! for, and every later command on that instance starts its provider with it.
//! A different endpoint on an existing instance is refused: another host is
//! another instance.
//!
//! The production CLI is copied beside a stand-in catalog provider
//! (`support/standin.rs`), which records the arguments it is started with. This
//! binary is that stand-in, so it runs without the standard test harness.
#[cfg(unix)]
#[path = "support/standin.rs"]
mod standin;

#[cfg(unix)]
fn main() {
    tests::main();
}

#[cfg(not(unix))]
fn main() {}

#[cfg(unix)]
mod tests {
    use super::standin::*;
    use serde_json::json;

    const URL: &str = "https://gitlab.example.test";
    const GITLAB: &[&str] = &[
        "connections",
        "connect",
        "gitlab/ref-1",
        "--profile",
        "gitlab.pat",
    ];

    pub fn main() {
        super::standin::main(&[
            (
                "the_endpoint_is_stored_and_reaches_the_provider",
                the_endpoint_is_stored_and_reaches_the_provider,
            ),
            (
                "an_instance_without_an_endpoint_gets_it",
                an_instance_without_an_endpoint_gets_it,
            ),
            (
                "a_different_endpoint_on_an_existing_instance_is_refused_and_the_file_is_unchanged",
                a_different_endpoint_on_an_existing_instance_is_refused_and_the_file_is_unchanged,
            ),
            (
                "the_target_is_the_connector_and_instance_options",
                the_target_is_the_connector_and_instance_options,
            ),
            (
                "a_connector_without_a_default_endpoint_is_refused_without_one",
                a_connector_without_a_default_endpoint_is_refused_without_one,
            ),
            (
                "an_endpoint_the_configuration_refuses_is_refused_without_writing",
                an_endpoint_the_configuration_refuses_is_refused_without_writing,
            ),
            (
                "no_other_command_takes_an_endpoint",
                no_other_command_takes_an_endpoint,
            ),
        ]);
    }

    fn with(args: &[&str], more: &[&str]) -> Vec<String> {
        args.iter()
            .chain(more)
            .map(|arg| (*arg).to_owned())
            .collect()
    }

    fn run(installation: &Installation, args: &[String]) -> std::process::Output {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        installation.run(&args)
    }

    /// story:connectors-do-short-invocation: `connections connect
    /// gitlab/ref-1 --endpoint <url>` on a configuration with no GitLab table
    /// creates `[connectors.gitlab]` enabled and the instance with the endpoint,
    /// keeps every other byte and the file owner-only, and goes on to the
    /// credential. A later command on `gitlab/ref-1`, which takes no
    /// `--endpoint`, starts the provider with that url.
    pub fn the_endpoint_is_stored_and_reaches_the_provider() {
        let installation = Installation::new();
        let before = installation.configure("");
        let (_, data) = refusal(&run(&installation, &with(GITLAB, &["--endpoint", URL])));
        // The connect went on past the configuration to the credential.
        assert_eq!(data["stage"], "protected_entry", "{data}");
        let text = installation.text();
        assert!(text.starts_with(&before), "{text}");
        let written = parsed(&text);
        assert_eq!(
            written["connectors"]["gitlab"]["enabled"],
            true.into(),
            "{text}"
        );
        assert_eq!(
            written["connectors"]["gitlab"]["instances"]["ref-1"]["endpoint"],
            URL.into(),
            "{text}"
        );
        assert_eq!(installation.mode(), 0o600);
        installation.forget_calls();
        let described = success(&installation.run(&[
            "instances",
            "describe",
            "--connector",
            "gitlab",
            "--instance",
            "ref-1",
        ]));
        let args = &described["configuration"]["executable"]["args"];
        assert_eq!(
            args,
            &json!([
                "--connector",
                "gitlab",
                "--instance",
                "ref-1",
                "--endpoint",
                URL
            ]),
            "{described}"
        );
        let calls = installation.calls();
        assert!(!calls.is_empty());
        for call in calls {
            assert_eq!(
                call,
                [
                    "--connector",
                    "gitlab",
                    "--instance",
                    "ref-1",
                    "--endpoint",
                    URL,
                    "--print-local-bootstrap"
                ]
            );
        }
    }

    /// story:connectors-do-short-invocation: an instance listed without an
    /// endpoint gets the key, and nothing else of its table changes.
    pub fn an_instance_without_an_endpoint_gets_it() {
        let installation = Installation::new();
        installation.configure(
            "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.ref-1]\ndescription='team forge'\n",
        );
        let (_, data) = refusal(&run(&installation, &with(GITLAB, &["--endpoint", URL])));
        assert_eq!(data["stage"], "protected_entry", "{data}");
        let written = parsed(&installation.text());
        let instance = &written["connectors"]["gitlab"]["instances"]["ref-1"];
        assert_eq!(instance["endpoint"], URL.into());
        assert_eq!(instance["description"], "team forge".into());
    }

    /// story:connectors-do-short-invocation: a different endpoint on an
    /// existing instance is `lifecycle_conflict` at the configuration naming
    /// `endpoint`, and the file keeps every byte.
    pub fn a_different_endpoint_on_an_existing_instance_is_refused_and_the_file_is_unchanged() {
        let installation = Installation::new();
        let before = installation.configure(&format!(
            "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.ref-1]\nendpoint='{URL}'\n"
        ));
        let (code, data) = refusal(&run(
            &installation,
            &with(GITLAB, &["--endpoint", "https://other.example.test"]),
        ));
        assert_eq!(code, Some(1), "{data}");
        assert_eq!(
            (
                &data["code"],
                &data["stage"],
                &data["next_action"],
                &data["field"]
            ),
            (
                &json!("lifecycle_conflict"),
                &json!("configuration"),
                &json!("check_configuration"),
                &json!("endpoint")
            ),
            "{data}"
        );
        assert_eq!(installation.text(), before);
    }

    /// story:connectors-do-short-invocation: `<connector>/<instance>` selects
    /// as `--connector` and `--instance` do; both forms together, or a target
    /// that is not `<connector>/<instance>`, are `invalid_input` naming
    /// `target`.
    pub fn the_target_is_the_connector_and_instance_options() {
        let installation = Installation::new();
        installation.configure(&format!(
            "[connectors.gitlab]\nenabled=true\n[connectors.gitlab.instances.ref-1]\nendpoint='{URL}'\n"
        ));
        let positional = refusal(&run(&installation, &with(GITLAB, &[])));
        let options = refusal(&installation.run(&[
            "connections",
            "connect",
            "--connector",
            "gitlab",
            "--instance",
            "ref-1",
            "--profile",
            "gitlab.pat",
        ]));
        assert_eq!(positional, options);
        assert_eq!(positional.1["stage"], "protected_entry", "{positional:?}");
        for (target, extra) in [
            ("gitlab/ref-1", &["--connector", "gitlab"][..]),
            ("gitlab/ref-1", &["--instance", "ref-1"][..]),
            ("gitlab/ref-1", &["--adapter", "gitlab.ref-1"][..]),
            ("gitlab", &[][..]),
            ("gitlab/", &[][..]),
            ("/ref-1", &[][..]),
            ("gitlab/ref-1/x", &[][..]),
        ] {
            let mut args = with(
                &["connections", "connect", target, "--profile", "gitlab.pat"],
                extra,
            );
            args.extend(["--endpoint".to_owned(), URL.to_owned()]);
            let before = installation.text();
            let (code, data) = refusal(&run(&installation, &args));
            assert_eq!(code, Some(2), "{target} {extra:?}: {data}");
            assert_eq!(
                (&data["code"], &data["stage"], &data["field"]),
                (
                    &json!("invalid_input"),
                    &json!("arguments"),
                    &json!("target")
                ),
                "{target} {extra:?}: {data}"
            );
            assert_eq!(installation.text(), before);
        }
    }

    /// story:connectors-do-short-invocation: a connector whose declaration
    /// names no default endpoint (Jira) and an instance without one: connect
    /// without `--endpoint` is `invalid_input` naming `endpoint`, before any
    /// credential is asked for and without writing.
    pub fn a_connector_without_a_default_endpoint_is_refused_without_one() {
        for body in [
            "",
            "[connectors.jira]\nenabled=true\n[connectors.jira.instances.work]\ndescription='tickets'\n",
        ] {
            let installation = Installation::new();
            let before = installation.configure(body);
            let (code, data) = refusal(&installation.run(&[
                "connections",
                "connect",
                "jira/work",
                "--profile",
                "jira.pat",
            ]));
            assert_eq!(code, Some(2), "{body}: {data}");
            assert_eq!(
                (&data["code"], &data["stage"], &data["field"]),
                (
                    &json!("invalid_input"),
                    &json!("arguments"),
                    &json!("endpoint")
                ),
                "{body}: {data}"
            );
            assert_eq!(installation.text(), before);
            // With the endpoint, the instance is written and the connect goes on.
            let (_, data) = refusal(&installation.run(&[
                "connections",
                "connect",
                "jira/work",
                "--profile",
                "jira.pat",
                "--endpoint",
                "https://example.atlassian.test",
            ]));
            assert_eq!(data["stage"], "protected_entry", "{body}: {data}");
            assert_eq!(
                parsed(&installation.text())["connectors"]["jira"]["instances"]["work"]["endpoint"],
                "https://example.atlassian.test".into()
            );
        }
    }

    /// story:connectors-do-short-invocation: an endpoint the configuration
    /// would refuse (empty, whitespace, a control character, over 2048 bytes)
    /// is `invalid_input` naming `endpoint`; one the provider refuses is the
    /// configuration's refusal. Neither writes anything.
    pub fn an_endpoint_the_configuration_refuses_is_refused_without_writing() {
        let installation = Installation::new();
        let before = installation.configure("");
        let long = format!("https://{}", "a".repeat(2048));
        for endpoint in [
            "",
            "https://a b",
            "https://a\tb",
            "https://a\u{7}b",
            long.as_str(),
        ] {
            let (code, data) = refusal(&run(
                &installation,
                &with(GITLAB, &["--endpoint", endpoint]),
            ));
            assert_eq!(code, Some(2), "{endpoint:?}: {data}");
            assert_eq!(
                (&data["code"], &data["field"]),
                (&json!("invalid_input"), &json!("endpoint")),
                "{endpoint:?}: {data}"
            );
            assert_eq!(installation.text(), before, "{endpoint:?}");
        }
        let (_, data) = refusal(&run(
            &installation,
            &with(GITLAB, &["--endpoint", "https://refuse.example.test"]),
        ));
        assert_eq!(
            (&data["code"], &data["stage"]),
            (&json!("invalid_configuration"), &json!("configuration")),
            "{data}"
        );
        assert_eq!(installation.text(), before);
    }

    /// story:connectors-do-short-invocation: no other command takes
    /// `--endpoint`; the parser refuses it as an unknown argument.
    pub fn no_other_command_takes_an_endpoint() {
        let installation = Installation::new();
        installation.configure("");
        for args in [
            &[
                "connections",
                "reconnect",
                "--connector",
                "gitlab",
                "--instance",
                "ref-1",
                "--expected-revision",
                "r",
                "--endpoint",
                URL,
            ][..],
            &[
                "connections",
                "revalidate",
                "--connection",
                "c",
                "--expected-revision",
                "r",
                "--endpoint",
                URL,
            ][..],
            &[
                "instances",
                "describe",
                "--connector",
                "gitlab",
                "--instance",
                "ref-1",
                "--endpoint",
                URL,
            ][..],
            &[
                "operations",
                "list",
                "--connector",
                "gitlab",
                "--instance",
                "ref-1",
                "--endpoint",
                URL,
            ][..],
            &[
                "operations",
                "describe",
                "--connector",
                "gitlab",
                "--instance",
                "ref-1",
                "--operation",
                "x",
                "--endpoint",
                URL,
            ][..],
        ] {
            let output = installation.run(args);
            assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
            let refused: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(refused["error"]["code"], "cli_parse", "{args:?}: {refused}");
        }
    }
}
