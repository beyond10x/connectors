//! Adversary pass 1 on unit U2 of wave D218, story:connectors-do-short-invocation
//! (decisions D10 and D11): `connections connect <connector>/<instance>
//! --endpoint <url>` writes the endpoint into the operator's configuration, and
//! a refused `--expected-revision` names which revision it expects.
//!
//! Each case states what the decided design promises and drives the production
//! CLI against it: the configuration keeps every byte but the endpoint, whatever
//! TOML form or line ending the operator wrote it in; a name the configuration
//! cannot hold is the target's refusal, not the file's; concurrent writers lose
//! no endpoint they reported as written; no other command takes `--endpoint`;
//! a revision refusal reveals no revision and no secret.
//!
//! Two fixtures share this binary. The endpoint cases copy the CLI beside this
//! binary as the stand-in catalog provider (`support/standin.rs`); the provider
//! is started as `--connector <c> --instance <i> ... --print-local-bootstrap`,
//! which the standard harness refuses, so a constructor in `.init_array` (run
//! after the standard library's own argument capture at `.init_array.00099`)
//! serves those arguments before the harness starts. The revision cases run the
//! CLI, owner and adapter child against the read-only `mounted_secret` binding
//! in a jail (`support/mounted.rs`), whose adapter child is this binary filtered
//! to [`adversary_d218_u2_adapter_fixture`]. Linux only: the binding is.
#![cfg(target_os = "linux")]

#[path = "support/mounted.rs"]
mod mounted;
#[path = "support/standin.rs"]
mod standin;

use connectors_custody::mounted::fixture::{Jail, Map};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{MetadataExt, symlink},
};

/// story:connectors-do-short-invocation: serve the stand-in catalog provider
/// when this binary is started as one, before the test harness parses
/// arguments it would refuse.
#[used]
#[unsafe(link_section = ".init_array")]
static STANDIN_PROVIDER: extern "C" fn() = standin_provider;

extern "C" fn standin_provider() {
    let provider = std::env::args().any(|arg| {
        arg == "--print-local-bootstrap" || arg == "--print-profiles" || arg == "--connector"
    });
    if provider {
        standin::main(&[]);
        std::process::exit(0);
    }
}

const URL: &str = "https://gitlab.example.test";

fn connect(
    installation: &standin::Installation,
    target: &str,
    endpoint: &str,
) -> std::process::Output {
    installation.run(&[
        "connections",
        "connect",
        target,
        "--profile",
        "gitlab.pat",
        "--endpoint",
        endpoint,
    ])
}

/// The configuration `connectors-local/5` of this user, then `body`, with
/// every line ended by `eol`.
fn configuration(body: &str, eol: &str) -> String {
    format!(
        "format='connectors-local/5'\nowner_uid={}\n{body}",
        connectors_host::local::filesystem::uid()
    )
    .replace('\n', eol)
}

/// Replace the configuration's text, keeping its owner-only mode.
fn rewrite(installation: &standin::Installation, text: &str) -> String {
    installation.configure("");
    fs::write(installation.config(), text).unwrap();
    assert_eq!(installation.mode(), 0o600);
    text.to_owned()
}

/// `after` without its one line that ends in `endpoint = "<endpoint>"`, that
/// line's terminator included.
fn minus_endpoint_line(after: &str, endpoint: &str) -> String {
    let wanted = format!("endpoint = \"{endpoint}\"");
    let mut removed = false;
    after
        .split_inclusive('\n')
        .filter(|line| {
            let hit = !removed
                && line
                    .trim_end_matches(['\r', '\n'])
                    .trim_end()
                    .ends_with(&wanted);
            removed |= hit;
            !hit
        })
        .collect()
}

fn stored_endpoint(text: &str, connector: &str, instance: &str) -> Option<String> {
    let parsed = standin::parsed(text);
    parsed
        .get("connectors")?
        .get(connector)?
        .get("instances")?
        .get(instance)?
        .get("endpoint")?
        .as_str()
        .map(str::to_owned)
}

/// story:connectors-do-short-invocation (D10, "the write keeps every other
/// byte of the file"): a configuration written with CRLF line endings, as an
/// editor on Windows writes it, keeps every line ending it had; only the
/// endpoint line is added.
#[test]
fn a_crlf_configuration_keeps_its_line_endings() {
    let installation = standin::Installation::new();
    let before = rewrite(
        &installation,
        &configuration(
            "\n# the forge\n[connectors.gitlab]\nenabled = true\n\n[connectors.gitlab.instances.ref-1]\ndescription = 'team forge'\n",
            "\r\n",
        ),
    );
    let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", URL));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    let after = installation.text();
    assert_eq!(
        stored_endpoint(&after, "gitlab", "ref-1").as_deref(),
        Some(URL),
        "{after:?}"
    );
    assert_eq!(
        minus_endpoint_line(&after, URL),
        before,
        "\nafter: {after:?}"
    );
}

/// story:connectors-do-short-invocation (D10, "keeps every other byte"): a
/// configuration that begins with a UTF-8 byte order mark, which the
/// configuration's own read admits, keeps it.
#[test]
fn a_byte_order_mark_is_kept() {
    let installation = standin::Installation::new();
    let before = rewrite(
        &installation,
        &format!(
            "\u{feff}{}",
            configuration(
                "[connectors.gitlab]\nenabled = true\n[connectors.gitlab.instances.ref-1]\ndescription = 'team forge'\n",
                "\n"
            )
        ),
    );
    let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", URL));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    let after = installation.text();
    assert_eq!(
        stored_endpoint(after.trim_start_matches('\u{feff}'), "gitlab", "ref-1").as_deref(),
        Some(URL)
    );
    assert_eq!(
        minus_endpoint_line(&after, URL),
        before,
        "\nafter: {after:?}"
    );
}

/// story:connectors-do-short-invocation (D10): a file whose last line has no
/// newline keeps every byte it had, whether the endpoint is added to its last
/// table or a new table follows it.
#[test]
fn a_last_line_without_a_newline_is_kept() {
    for (body, target) in [
        (
            "[connectors.gitlab]\nenabled = true\n[connectors.gitlab.instances.ref-1]\ndescription = 'team forge'",
            "gitlab/ref-1",
        ),
        ("[connectors.jira]\nenabled = false", "gitlab/ref-1"),
    ] {
        let installation = standin::Installation::new();
        let before = rewrite(&installation, &configuration(body, "\n"));
        let (_, data) = standin::refusal(&connect(&installation, target, URL));
        assert_eq!(data["stage"], "protected_entry", "{body:?}: {data}");
        let after = installation.text();
        assert_eq!(
            stored_endpoint(&after, "gitlab", "ref-1").as_deref(),
            Some(URL),
            "{after:?}"
        );
        assert!(after.starts_with(&before), "{body:?}\nafter: {after:?}");
    }
}

/// story:connectors-do-short-invocation (D10): an instance written with dotted
/// keys at the root takes the endpoint as a dotted key beside them, every
/// other byte kept; another endpoint there is refused and the file unchanged;
/// the same endpoint again writes nothing.
#[test]
fn a_root_dotted_instance_takes_the_endpoint_in_place() {
    let installation = standin::Installation::new();
    let before = rewrite(
        &installation,
        &configuration(
            "connectors.gitlab.enabled = true # dotted\nconnectors.gitlab.instances.ref-1.description = 'team forge'\n",
            "\n",
        ),
    );
    let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", URL));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    let after = installation.text();
    assert_eq!(
        stored_endpoint(&after, "gitlab", "ref-1").as_deref(),
        Some(URL),
        "{after:?}"
    );
    assert_eq!(
        minus_endpoint_line(&after, URL),
        before,
        "\nafter: {after:?}"
    );
    let inode = fs::metadata(installation.config()).unwrap().ino();
    let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", URL));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    assert_eq!(fs::metadata(installation.config()).unwrap().ino(), inode);
    let (code, data) = standin::refusal(&connect(
        &installation,
        "gitlab/ref-1",
        "https://other.example.test",
    ));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        (&data["code"], &data["field"]),
        (&json!("lifecycle_conflict"), &json!("endpoint")),
        "{data}"
    );
    assert_eq!(installation.text(), after);
}

/// story:connectors-do-short-invocation (D10): an instance written as an
/// inline table takes the endpoint inside it; every other line is kept.
#[test]
fn an_inline_instance_takes_the_endpoint_inline() {
    let installation = standin::Installation::new();
    let before = rewrite(
        &installation,
        &configuration(
            "[connectors.gitlab]\nenabled = true\ninstances = { ref-1 = { description = 'team forge' } }\n# after\n",
            "\n",
        ),
    );
    let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", URL));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    let after = installation.text();
    assert_eq!(
        stored_endpoint(&after, "gitlab", "ref-1").as_deref(),
        Some(URL),
        "{after:?}"
    );
    let other = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| !line.starts_with("instances"))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(other(&after), other(&before), "\nafter: {after:?}");
}

/// story:connectors-do-short-invocation (D10, "a target that is not
/// `<connector>/<instance>`" is `invalid_input` naming `target`): a connector
/// or instance name the configuration can never hold is the argument's
/// refusal, at the arguments, naming `target`; it is not reported as the
/// operator's configuration being invalid, and nothing is written.
#[test]
fn a_name_outside_the_selector_grammar_is_the_targets_refusal() {
    let installation = standin::Installation::new();
    let before = rewrite(
        &installation,
        &configuration("[connectors.gitlab]\nenabled = true\n", "\n"),
    );
    let long = format!("gitlab/{}", "r".repeat(129));
    let mut wrong = Vec::new();
    for target in [
        "gitlab/ref.1",
        "gitlab/ref 1",
        "git.lab/ref-1",
        "gitlab/réf",
        long.as_str(),
    ] {
        let (code, data) = standin::refusal(&connect(&installation, target, URL));
        assert_eq!(installation.text(), before, "{target:?}");
        if (code, &data["code"], &data["stage"], &data["field"])
            != (
                Some(2),
                &json!("invalid_input"),
                &json!("arguments"),
                &json!("target"),
            )
        {
            wrong.push((target.chars().take(24).collect::<String>(), code, data));
        }
    }
    assert!(
        wrong.is_empty(),
        "expected invalid_input at arguments naming target:\n{wrong:#?}"
    );
}

/// story:connectors-do-short-invocation (D10, "the write ... is atomic"):
/// concurrent connects to distinct instances may refuse one another, but every
/// connect that went on to its credential finds its endpoint in the file, the
/// file stays owner-only and parses, and no temporary file is left.
#[test]
fn concurrent_connects_lose_no_endpoint_they_went_on_with() {
    let installation = standin::Installation::new();
    rewrite(
        &installation,
        &configuration("[connectors.gitlab]\nenabled = true\n", "\n"),
    );
    let outcomes: Vec<(String, Value)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..12)
            .map(|n| {
                let installation = &installation;
                scope.spawn(move || {
                    let endpoint = format!("https://forge-{n}.example.test");
                    let (_, data) = standin::refusal(&connect(
                        installation,
                        &format!("gitlab/ref-{n}"),
                        &endpoint,
                    ));
                    (format!("ref-{n}"), data)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let text = installation.text();
    assert_eq!(installation.mode(), 0o600);
    let went_on: Vec<_> = outcomes
        .iter()
        .filter(|(_, data)| data["stage"] == "protected_entry")
        .collect();
    assert!(!went_on.is_empty(), "{outcomes:?}");
    for (instance, _) in &went_on {
        let n = instance.trim_start_matches("ref-");
        assert_eq!(
            stored_endpoint(&text, "gitlab", instance).as_deref(),
            Some(format!("https://forge-{n}.example.test").as_str()),
            "{instance} went on but its endpoint is not stored\n{text}\n{outcomes:?}"
        );
    }
    let names: Vec<_> = fs::read_dir(installation.config().parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "config.toml")
        .collect();
    assert!(names.is_empty(), "{names:?}");
}

/// story:connectors-do-short-invocation (D10, "another host is another
/// instance"): concurrent connects giving one instance different endpoints;
/// every connect that went on to its credential went on with the endpoint the
/// file holds.
#[test]
fn concurrent_endpoints_on_one_instance_agree_with_the_file() {
    let installation = standin::Installation::new();
    rewrite(
        &installation,
        &configuration("[connectors.gitlab]\nenabled = true\n", "\n"),
    );
    let outcomes: Vec<(String, Value)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..12)
            .map(|n| {
                let installation = &installation;
                scope.spawn(move || {
                    let endpoint = format!("https://forge-{n}.example.test");
                    let (_, data) =
                        standin::refusal(&connect(installation, "gitlab/ref-1", &endpoint));
                    (endpoint, data)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let stored = stored_endpoint(&installation.text(), "gitlab", "ref-1");
    for (endpoint, data) in &outcomes {
        if data["stage"] == "protected_entry" {
            assert_eq!(stored.as_deref(), Some(endpoint.as_str()), "{outcomes:?}");
        }
    }
}

/// story:connectors-do-short-invocation (D10, "admitted by the same rules the
/// configuration applies"): the bound is 2048 bytes, inclusive. The unit's own
/// case tries 2056 bytes only, so a bound moved to 2047 or 2055 stays green
/// there.
#[test]
fn an_endpoint_of_exactly_2048_bytes_is_written_and_2049_is_refused() {
    let installation = standin::Installation::new();
    let before = rewrite(&installation, &configuration("", "\n"));
    let longer = format!("https://{}", "a".repeat(2049 - 8));
    let (code, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", &longer));
    assert_eq!(code, Some(2), "{data}");
    assert_eq!(
        (&data["code"], &data["field"]),
        (&json!("invalid_input"), &json!("endpoint")),
        "{data}"
    );
    assert_eq!(installation.text(), before);
    let exact = format!("https://{}", "a".repeat(2048 - 8));
    assert_eq!(exact.len(), 2048);
    let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", &exact));
    assert_eq!(data["stage"], "protected_entry", "{data}");
    assert_eq!(
        stored_endpoint(&installation.text(), "gitlab", "ref-1"),
        Some(exact)
    );
}

/// story:connectors-do-short-invocation (D10): an admitted endpoint holding
/// what TOML must escape or quote is stored as given and reaches the provider
/// as given.
#[test]
fn an_endpoint_toml_must_escape_reaches_the_provider_as_given() {
    for endpoint in [
        "https://gitlab.example.test/a\"b",
        "https://gitlab.example.test/a\\b",
        "https://gitlab.example.test/a'b#c]d",
        "https://gitläb.example.test/\u{200b}",
        "https://gitlab.example.test/\"\"\"'''",
    ] {
        let installation = standin::Installation::new();
        rewrite(&installation, &configuration("", "\n"));
        let (_, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", endpoint));
        assert_eq!(data["stage"], "protected_entry", "{endpoint:?}: {data}");
        let text = installation.text();
        assert_eq!(
            stored_endpoint(&text, "gitlab", "ref-1").as_deref(),
            Some(endpoint),
            "{text}"
        );
        let calls = installation.calls();
        assert!(!calls.is_empty());
        for call in calls {
            let at = call.iter().position(|arg| arg == "--endpoint").unwrap();
            assert_eq!(call[at + 1], endpoint, "{call:?}");
        }
    }
}

/// story:connectors-do-short-invocation (D10, "the write ... refuses a file
/// that is no configuration"; `Config::read` its size bound): an edit that
/// would carry the file past the configuration's size bound is refused and
/// the file is unchanged.
#[test]
fn an_edit_past_the_size_bound_is_refused_unchanged() {
    let installation = standin::Installation::new();
    let head = configuration("[connectors.gitlab]\nenabled = true\n", "\n");
    let limit = connectors_host::local::config::CONFIG_LIMIT;
    let pad = limit - head.len() - 3;
    let before = rewrite(&installation, &format!("{head}#{}\n", "x".repeat(pad)));
    assert_eq!(before.len(), limit - 1);
    let (code, data) = standin::refusal(&connect(&installation, "gitlab/ref-1", URL));
    assert_eq!(code, Some(2), "{data}");
    assert_eq!(
        (&data["code"], &data["configuration_rule"]),
        (&json!("invalid_configuration"), &json!("config_too_large")),
        "{data}"
    );
    assert_eq!(installation.text(), before);
}

/// story:connectors-do-short-invocation (D10): a configuration directory that
/// is a symbolic link is either refused, leaving what it names unchanged, or
/// read and written through consistently: the file it names takes the
/// endpoint, stays owner-only, and the link stays a link.
#[test]
fn a_symlinked_configuration_directory_is_read_and_written_alike() {
    let installation = standin::Installation::new();
    let before = rewrite(&installation, &configuration("", "\n"));
    let directory = installation.config().parent().unwrap().to_owned();
    let real = installation.root.path().join("real-config");
    fs::rename(&directory, &real).unwrap();
    symlink(&real, &directory).unwrap();
    let output = connect(&installation, "gitlab/ref-1", URL);
    let (code, data) = standin::refusal(&output);
    let text = fs::read_to_string(real.join("config.toml")).unwrap();
    assert!(
        fs::symlink_metadata(&directory)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    if data["stage"] == "protected_entry" {
        assert_eq!(
            stored_endpoint(&text, "gitlab", "ref-1").as_deref(),
            Some(URL),
            "{text}"
        );
        assert_eq!(
            fs::metadata(real.join("config.toml")).unwrap().mode() & 0o777,
            0o600
        );
    } else {
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(data["code"], "invalid_configuration", "{data}");
        assert_eq!(text, before);
    }
}

/// The command paths and aliases of the `ess-cli/1` binding.
fn binding_paths() -> Vec<Vec<String>> {
    let spec = include_str!("../spec/cli.yaml");
    let words = |list: &str| -> Vec<String> {
        list.split(',')
            .map(|word| word.trim().trim_matches(['[', ']']).trim().to_owned())
            .filter(|word| !word.is_empty())
            .collect()
    };
    let mut paths = Vec::new();
    for line in spec.lines() {
        if let Some(path) = line.strip_prefix("  - path: ") {
            paths.push(words(path));
        } else if let Some(aliases) = line.strip_prefix("    aliases: ") {
            let aliases = aliases.trim().trim_start_matches('[').trim_end_matches(']');
            for alias in aliases
                .split("], [")
                .filter(|alias| !alias.trim().is_empty())
            {
                paths.push(words(alias));
            }
        }
    }
    paths
}

/// story:connectors-do-short-invocation (D10, "No other command takes
/// `--endpoint`"): every command path and alias of the binding but
/// `connections connect`, and the global position before any command, refuse
/// `--endpoint` as an unknown argument, and nothing is written.
#[test]
fn no_command_but_connect_takes_an_endpoint() {
    let installation = standin::Installation::new();
    let before = rewrite(&installation, &configuration("", "\n"));
    let paths = binding_paths();
    assert!(paths.len() > 30, "{paths:?}");
    let mut taken = Vec::new();
    for path in paths
        .iter()
        .filter(|path| *path != &["connections", "connect"])
    {
        let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
        args.extend(["--endpoint", URL]);
        let output = installation.run(&args);
        let refused: Value = serde_json::from_slice(&output.stderr).unwrap_or(Value::Null);
        if output.status.code() != Some(2) || refused["error"]["code"] != "cli_parse" {
            taken.push((path.join(" "), output.status.code(), refused));
        }
    }
    let output = installation.run(&["--endpoint", URL, "connections", "list"]);
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap_or(Value::Null);
    if output.status.code() != Some(2) || refused["error"]["code"] != "cli_parse" {
        taken.push((
            "--endpoint before the command".into(),
            output.status.code(),
            refused,
        ));
    }
    assert!(taken.is_empty(), "{taken:#?}");
    assert_eq!(installation.text(), before);
}

/// The adapter child the jailed owner starts: this binary, filtered to this test.
#[test]
fn adversary_d218_u2_adapter_fixture() {
    mounted::serve_when_child();
}

/// A jail with one connection of `forge`: the container, its reference and revision.
fn connected() -> (mounted::Container, String, String) {
    let container = mounted::Container::new("adversary_d218_u2_adapter_fixture");
    container.deliver(mounted::PROFILE, mounted::SECRET);
    container.configure("connectors-local/4", &mounted::Container::mounted());
    let init = mounted::success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = mounted::success(&container.connect(mounted::PROFILE));
    let summary = &connected["connection"]["summary"];
    (
        container,
        summary["connection"].as_str().unwrap().to_owned(),
        summary["revision"].as_str().unwrap().to_owned(),
    )
}

/// The configuration revision `forge` is configured under (`mounted.rs`).
const CONFIGURATION_REVISION: &str = "cfg-1";

/// story:connectors-do-short-invocation (D11): no revision refusal of
/// `revalidate` or `revoke`, given the configuration revision or a stale one,
/// in JSON or text, reveals the connection's actual revision or the secret.
#[test]
fn revision_refusals_reveal_no_revision_and_no_secret() {
    let (container, connection, revision) = connected();
    let secret = String::from_utf8_lossy(mounted::SECRET).into_owned();
    let mut seen = Vec::new();
    for verb in ["revalidate", "revoke"] {
        for given in [CONFIGURATION_REVISION, "0000000000000000-stale"] {
            for output in ["json", "human"] {
                let run = Jail::output(
                    container
                        .jail
                        .command("/bin-test/connectors", Map::Own)
                        .args(["--output", output, "--config", mounted::CONFIG])
                        .args(["--state-dir", mounted::STATE, "connections", verb])
                        .args(["--adapter", mounted::ALIAS, "--connection", &connection])
                        .args(["--expected-revision", given])
                        .env_clear()
                        .stdin(std::process::Stdio::null()),
                );
                assert!(!run.status.success(), "{verb} {given}: {run:?}");
                let printed = format!(
                    "{}{}",
                    String::from_utf8_lossy(&run.stdout),
                    String::from_utf8_lossy(&run.stderr)
                );
                assert!(
                    !printed.contains(&revision),
                    "{verb} {given} {output}: {printed}"
                );
                assert!(
                    !printed.contains(&secret),
                    "{verb} {given} {output}: {printed}"
                );
                seen.push(printed);
            }
        }
    }
    assert!(
        seen.iter()
            .all(|printed| printed.contains("expected_revision")),
        "{seen:#?}"
    );
}

/// story:connectors-do-short-invocation (D11, "Apply the same to `connections
/// reconnect` ..."): reconnect takes the same `--expected-revision` as
/// revalidate and revoke; given the instance's configuration revision it is
/// refused the way they are, naming the argument, the revision it takes, the
/// one given and where to read the right one. Nothing is captured.
#[test]
fn reconnect_given_the_configuration_revision_names_the_connection_revision() {
    let (container, connection, revision) = connected();
    assert_ne!(revision, CONFIGURATION_REVISION);
    let (code, data) = mounted::refusal(&container.run(&[
        "connections",
        "reconnect",
        "--adapter",
        mounted::ALIAS,
        "--connection",
        &connection,
        "--expected-revision",
        CONFIGURATION_REVISION,
    ]));
    assert_eq!(
        (
            code,
            &data["code"],
            &data["field"],
            &data["revision_expected"],
            &data["revision_given"],
            &data["read_with"],
        ),
        (
            Some(2),
            &json!("invalid_input"),
            &json!("expected_revision"),
            &json!("connection"),
            &json!("configuration"),
            &json!("connections describe"),
        ),
        "{data}"
    );
    let described = mounted::success(&container.run(&[
        "connections",
        "describe",
        "--adapter",
        mounted::ALIAS,
        "--connection",
        &connection,
    ]));
    assert_eq!(
        described["connection"]["summary"]["revision"],
        revision.as_str()
    );
}
