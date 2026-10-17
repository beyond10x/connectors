//! Adversary cases for story:connections-reconnect-verb.
//!
//! `connections repair` and `adapters list|describe|status|stop` stay path
//! aliases of `connections reconnect` and `instances …`
//! (`contracts/cli/v1alpha1/semantics.md` §3 "Renamed commands"). These cases
//! hold the old and new names to the same answer on argument combinations the
//! unit's own tests do not run (a stale `--expected-revision`, each protected
//! source, two sources at once), hold the `--output json` standard error of
//! the old names to one envelope or nothing for help and usage errors, and
//! hold the generated CLI reference to the renamed paths.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

#[cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;

/// The repository root.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned()
}

fn plan() -> connectors_cli_contract::wire::Plan {
    connectors_cli_contract::plan()
}

/// `website/docs/reference/cli.md` is generated from the clap tree the binary
/// parses (`crates/connectors-docs/src/cli.rs`) and the gate fails on drift
/// (AGENTS.md, `connectors-docs generate --check`). Every command path the
/// plan declares has its section there. The new path's section carries the
/// plan's `about`; an old path's section names it the deprecated name of the
/// new one rather than repeating that `about` (story:cli-text-without-adapter,
/// coordinator additions: an alias listed with the new command's about and no
/// deprecation mark is what that story fixes).
#[test]
fn the_cli_reference_page_has_a_section_for_every_renamed_path() {
    let page = fs::read_to_string(repository().join("website/docs/reference/cli.md")).unwrap();
    let mut missing = Vec::new();
    for command in plan().commands {
        let renamed = !command.aliases.is_empty();
        if !renamed {
            continue;
        }
        let deprecated = format!("Deprecated name of `connectors {}`", command.path.join(" "));
        for (path, about) in std::iter::once((&command.path, &command.about))
            .chain(command.aliases.iter().map(|alias| (alias, &deprecated)))
        {
            let heading = format!("## `connectors {}`", path.join(" "));
            match page.find(&heading) {
                None => missing.push(format!("{heading}: no section")),
                Some(at) => {
                    let section = &page[at + heading.len()..];
                    let section = &section[..section.find("\n## ").unwrap_or(section.len())];
                    if !section.contains(about.as_str()) {
                        missing.push(format!("{heading}: about is not {about:?}"));
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

/// The model reference of `connectors.cli` is generated from
/// `ess/domains/cli.yaml` by the pinned `ess generate --kind docs`; the
/// renamed `ConnectionReconnect*` types appear there and the old names do not.
#[test]
fn the_model_reference_names_the_reconnect_types() {
    let page = fs::read_to_string(
        repository().join("website/docs/reference/contracts/model/domains/connectors-cli.md"),
    )
    .unwrap();
    for name in [
        "ConnectionReconnectInput",
        "ConnectionReconnectResult",
        "ConnectionReconnectRequest",
    ] {
        assert!(page.contains(name), "{name} is not in the model reference");
    }
    assert!(
        !page.contains("ConnectionRepair"),
        "the model reference still names ConnectionRepair*"
    );
}

/// Under `--output json`, help is stdout with nothing on standard error, and a
/// usage error is exactly one failure envelope on standard error with nothing
/// on stdout, under an old name as under a new one.
#[test]
fn json_help_and_usage_errors_of_old_names_keep_the_stream_contract() {
    let root = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| -> Output {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args(["--output", "json", "--config"])
            .arg(root.path().join("config.toml"))
            .arg("--state-dir")
            .arg(root.path().join("state"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let mut wrong = Vec::new();
    for args in [
        &["adapters", "--help"][..],
        &["connections", "repair", "--help"],
        &["adapters", "list", "--help"],
        &["adapters", "stop", "-h"],
        &["help", "adapters", "list"],
        &["help", "connections", "repair"],
        &["adapters", "help", "describe"],
    ] {
        let output = run(args);
        if output.status.code() != Some(0) || !output.stderr.is_empty() || output.stdout.is_empty()
        {
            wrong.push(format!("{args:?}: {output:?}"));
        }
    }
    for args in [
        &["adapters"][..],
        &["adapters", "lis"],
        &["adapters", "list", "--no-such-flag"],
        &["connections", "repair"],
        &[
            "connections",
            "repair",
            "--connection",
            "c",
            "--expected-revision",
        ],
        &[
            "connections",
            "repair",
            "--connection",
            "c",
            "--expected-revision",
            "r",
            "--credential-file",
            "f",
            "--credential-stdin",
        ],
        &["adapters", "stop", "--connector", "x"],
    ] {
        let output = run(args);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let one = stderr.lines().count() == 1
            && serde_json::from_str::<Value>(&stderr).is_ok_and(|v| v["ok"] == false);
        if output.status.code() != Some(2) || !output.stdout.is_empty() || !one {
            wrong.push(format!("{args:?}: {output:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The deprecation follows the path clap selects, wherever the process
/// globals stand: between the group and the leaf, with `=` or without.
#[test]
fn the_deprecation_follows_the_parsed_path_wherever_the_globals_stand() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    let state = root.path().join("state");
    let (config, state) = (config.to_str().unwrap(), state.to_str().unwrap());
    let run = |args: &[&str]| -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let text = if output.status.success() {
            &output.stdout
        } else {
            &output.stderr
        };
        serde_json::from_slice(text).unwrap_or_else(|_| panic!("{args:?}: {output:?}"))
    };
    let config_eq = format!("--config={config}");
    let state_eq = format!("--state-dir={state}");
    let entry = json!([{"command":"adapters list","replacement":["instances list"]}]);
    for args in [
        vec![
            "adapters",
            "--output",
            "json",
            "--config",
            config,
            "list",
            "--state-dir",
            state,
        ],
        vec!["--output=json", &config_eq, "adapters", &state_eq, "list"],
        vec![
            "adapters",
            "--output=json",
            "--state-dir",
            state,
            "--config",
            config,
            "list",
            "--limit",
            "1",
        ],
    ] {
        assert_eq!(run(&args)["deprecations"], entry, "{args:?}");
    }
    for args in [
        vec![
            "instances",
            "--output",
            "json",
            "--config",
            config,
            "list",
            "--state-dir",
            state,
        ],
        vec!["--output=json", &config_eq, "instances", &state_eq, "list"],
    ] {
        assert!(run(&args).get("deprecations").is_none(), "{args:?}");
    }
}

#[cfg(target_os = "linux")]
mod reconnect {
    use super::mounted::*;
    use super::*;
    use connectors_custody::mounted::fixture::{Jail, Map};

    const ADAPTER_FIXTURE: &str = "reconnect::renamed_adversary_adapter_fixture";

    /// The adapter child the owner starts: this binary, filtered to this test.
    #[test]
    fn renamed_adversary_adapter_fixture() {
        serve_when_child();
    }

    fn copy_tree(from: &Path, to: &Path) {
        use std::os::unix::fs::FileTypeExt;
        fs::create_dir(to).unwrap();
        fs::set_permissions(to, fs::metadata(from).unwrap().permissions()).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            let target = to.join(entry.file_name());
            if kind.is_dir() {
                copy_tree(&entry.path(), &target);
            } else if kind.is_file() {
                fs::copy(entry.path(), &target).unwrap();
            } else {
                assert!(kind.is_socket(), "{:?}", entry.path());
            }
        }
    }

    fn steady(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                for (key, field) in fields.iter_mut() {
                    if key.ends_with("_ms") && field.is_i64() {
                        *field = json!(0);
                    } else {
                        steady(field);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(steady),
            _ => {}
        }
    }

    /// Text with every run of digits as one `0`: observation times differ.
    fn digits(text: &str) -> String {
        let mut out = String::new();
        let mut previous = false;
        for c in text.chars() {
            let digit = c.is_ascii_digit();
            if !digit || !previous {
                out.push(if digit { '0' } else { c });
            }
            previous = digit;
        }
        out
    }

    fn run(container: &Container, output: &str, args: &[&str]) -> Output {
        Jail::output(
            container
                .jail
                .command("/bin-test/connectors", Map::Own)
                .args(["--output", output, "--config", CONFIG, "--state-dir", STATE])
                .args(args)
                .env_clear()
                .stdin(Stdio::null()),
        )
    }

    /// One envelope: stdout on success with an empty standard error, standard
    /// error on failure with an empty stdout.
    fn envelope(output: &Output) -> Value {
        let (text, other) = if output.status.success() {
            (&output.stdout, &output.stderr)
        } else {
            (&output.stderr, &output.stdout)
        };
        assert!(other.is_empty(), "{output:?}");
        let mut value: Value =
            serde_json::from_slice(text).unwrap_or_else(|_| panic!("{output:?}"));
        steady(&mut value);
        value
    }

    /// From one connected state, each argument combination of `connections
    /// reconnect` answers under `connections repair` with the same exit
    /// status and envelope, the old name's carrying one more `deprecations`
    /// entry first; under `--output human` the same stdout, and standard error
    /// with one line more ahead.
    #[test]
    fn reconnect_and_repair_answer_alike_on_every_argument_combination() {
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        container.configure("connectors-local/4", &Container::mounted());
        success(&container.run(&["setup", "init"]));
        let connected = success(&container.connect(PROFILE));
        let summary = &connected["connection"]["summary"];
        let connection = summary["connection"].as_str().unwrap().to_owned();
        let revision = summary["revision"].as_str().unwrap().to_owned();
        let file = container.path("/srv/connectors/credential");
        fs::write(&file, b"fictional-file-token").unwrap();
        fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
        container.rotate(
            "..2026_10_15_00_00_00.000000009",
            b"fictional-mounted-token-0009",
        );
        success(&container.run(&["setup", "owner-stop"]));
        let state = container.path(STATE);
        let saved = container.path("/srv/connectors/saved-state");
        copy_tree(&state, &saved);

        let stale = format!("{revision}0");
        let target: Vec<&str> = vec!["--connection", &connection];
        let cases: Vec<(&str, Vec<&str>)> = vec![
            ("current", vec!["--expected-revision", &revision]),
            ("stale revision", vec!["--expected-revision", &stale]),
            (
                "unknown revision",
                vec!["--expected-revision", "rev-unknown"],
            ),
            (
                "file",
                vec![
                    "--expected-revision",
                    &revision,
                    "--credential-file",
                    "/srv/connectors/credential",
                ],
            ),
            (
                "stdin",
                vec!["--expected-revision", &revision, "--credential-stdin"],
            ),
            (
                "prompt",
                vec!["--expected-revision", &revision, "--credential-prompt"],
            ),
            (
                "two sources",
                vec![
                    "--expected-revision",
                    &revision,
                    "--credential-file",
                    "/srv/connectors/credential",
                    "--credential-stdin",
                ],
            ),
        ];
        let mut wrong = Vec::new();
        let mut outcomes = Vec::new();
        for (name, rest) in &cases {
            for selector in [
                vec!["--connector", "catalog", "--instance", ALIAS],
                vec!["--adapter", ALIAS],
            ] {
                let mut answers = Vec::new();
                for (verb, mode) in [
                    ("reconnect", "json"),
                    ("repair", "json"),
                    ("reconnect", "human"),
                    ("repair", "human"),
                ] {
                    fs::remove_dir_all(&state).unwrap();
                    copy_tree(&saved, &state);
                    let mut args = vec!["connections", verb];
                    args.extend(&selector);
                    args.extend(&target);
                    args.extend(rest);
                    let output = run(&container, mode, &args);
                    let _ = container.run(&["setup", "owner-stop"]);
                    answers.push(output);
                }
                let label = format!("{name} {selector:?}");
                let (new, old) = (&answers[0], &answers[1]);
                if new.status.code() != old.status.code() {
                    wrong.push(format!(
                        "{label}: exit {:?} vs {:?}",
                        new.status, old.status
                    ));
                    continue;
                }
                let first = envelope(new);
                let mut second = envelope(old);
                let deprecations = second
                    .as_object_mut()
                    .unwrap()
                    .remove("deprecations")
                    .unwrap_or_default();
                let mut expected = vec![
                    json!({"command":"connections repair","replacement":["connections reconnect"]}),
                ];
                expected.extend(
                    first
                        .get("deprecations")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                );
                let mut first_bare = first.clone();
                first_bare.as_object_mut().unwrap().remove("deprecations");
                if deprecations != json!(expected) || first_bare != second {
                    wrong.push(format!("{label}:\n  {first}\n  {second} + {deprecations}"));
                }
                outcomes.push(format!(
                    "{label}: {}",
                    if first["ok"] == true {
                        "ok".to_owned()
                    } else {
                        format!(
                            "{}/{}",
                            first["error"]["code"], first["error"]["data"]["code"]
                        )
                    }
                ));
                let (new, old) = (&answers[2], &answers[3]);
                let (new_err, old_err) = (
                    String::from_utf8_lossy(&new.stderr).into_owned(),
                    String::from_utf8_lossy(&old.stderr).into_owned(),
                );
                let line = "deprecated: connections repair; use connections reconnect\n";
                if new.status.code() != old.status.code()
                    || new.stdout.is_empty() != old.stdout.is_empty()
                    || digits(&old_err) != digits(&format!("{line}{new_err}"))
                {
                    wrong.push(format!("{label} human:\n  {new:?}\n  {old:?}"));
                }
            }
        }
        println!("{}", outcomes.join("\n"));
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
