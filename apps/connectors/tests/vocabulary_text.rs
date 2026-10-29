//! story:cli-text-without-adapter: the text a user reads speaks the connector
//! vocabulary (`docs/design/connector-vocabulary.md`). The checked text is the
//! standard output of `connectors --help`, of `connectors <path> --help` for
//! every group, command path and deprecated alias of the binding, of
//! `connectors-catalog-provider --help`, and the standard error the CLI renders
//! under `--output human` for every variant of `connectors.cli.FailureCode`,
//! `connectors.cli.FailureStage` and `connectors.cli.NextAction`.
//!
//! A line of it may say "adapter" (any case) only where it names the deprecated
//! `--adapter` option or the deprecated `adapters` command path, and only when
//! the same line says `deprecated`. Wire values and type names are not checked
//! text: JSON `adapter`, callable ids `adapters-*`, `connectors.cli.Adapter*`.
use connectors_cli_contract::{
    AcquireError, Handler, HandlerReply, Invocation, ProtectedSource, Sources,
    wire::{Plan, Shape},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::PathBuf,
    process::{Command, Output},
};

fn cli(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Every path that has help: the root, each group, each command path and each
/// alias of the binding, in the binding's order.
fn help_paths(plan: &Plan) -> Vec<Vec<String>> {
    let mut paths = vec![Vec::new()];
    let mut seen = BTreeSet::new();
    for command in &plan.commands {
        for path in std::iter::once(&command.path).chain(&command.aliases) {
            for depth in 1..=path.len() {
                let prefix = path[..depth].to_vec();
                if seen.insert(prefix.clone()) {
                    paths.push(prefix);
                }
            }
        }
    }
    paths
}

/// The lines of `text` that say "adapter" outside the allowlist.
fn refused_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            if !lower.contains("adapter") {
                return false;
            }
            let rest = lower.replace("--adapter", " ");
            let other = rest
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .any(|word| word.contains("adapter") && word != "adapters");
            other || !lower.contains("deprecated")
        })
        .map(str::to_owned)
        .collect()
}

/// `connectors-catalog-provider`, built beside `connectors` in the same
/// profile directory. A workspace test build builds it; a package-scoped one
/// needs `cargo build -p connectors-catalog-provider` first.
fn catalog_provider() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_connectors")).with_file_name(format!(
        "connectors-catalog-provider{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        path.is_file(),
        "{} is not built; run `cargo build -p connectors-catalog-provider` first",
        path.display()
    );
    path
}

#[test]
fn help_says_adapter_only_on_a_deprecated_line() {
    let plan = connectors_cli_contract::plan();
    let mut refused = Vec::new();
    let paths = help_paths(&plan);
    for path in &paths {
        let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
        args.push("--help");
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        for line in refused_lines(&text(&output.stdout)) {
            refused.push(format!("connectors {}: {line}", args.join(" ")));
        }
    }
    let provider = Command::new(catalog_provider())
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(provider.status.code(), Some(0), "{provider:?}");
    for line in refused_lines(&text(&provider.stdout)) {
        refused.push(format!("connectors-catalog-provider --help: {line}"));
    }
    assert!(refused.is_empty(), "{}", refused.join("\n"));
    // Root, 6 groups, 33 command paths and 5 aliases: `invoke` is one, and
    // `operations invoke` is removed (story:connectors-do-short-invocation).
    assert_eq!(paths.len(), 45, "{paths:?}");
}

/// The listing line `help` writes for `name`, the words after the name.
fn listed<'a>(help: &'a str, name: &str) -> Option<&'a str> {
    help.lines().find_map(|line| {
        let mut words = line.trim_start().splitn(2, ' ');
        (words.next() == Some(name)).then(|| words.next().unwrap_or("").trim_start())
    })
}

#[test]
fn root_and_group_help_mark_each_deprecated_path() {
    let plan = connectors_cli_contract::plan();
    let canonical: BTreeSet<&str> = plan
        .commands
        .iter()
        .map(|command| command.path[0].as_str())
        .collect();
    // Each deprecated name, as the parent that lists it and its own word. A
    // word an alias shares with a command path is that path's.
    let deprecated: BTreeSet<(Vec<&str>, &str)> = plan
        .commands
        .iter()
        .flat_map(|command| &command.aliases)
        .flat_map(|alias| {
            (1..=alias.len()).map(move |depth| {
                (
                    alias[..depth - 1]
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                    alias[depth - 1].as_str(),
                )
            })
        })
        .filter(|(parent, name)| !(parent.is_empty() && canonical.contains(name)))
        .collect();
    let mut unmarked = Vec::new();
    for (parent, name) in &deprecated {
        let output = cli(&[&parent[..], &["--help"]].concat());
        let help = text(&output.stdout);
        match listed(&help, name) {
            Some(about) if about.starts_with("Deprecated") => {}
            about => unmarked.push(format!(
                "connectors {} --help lists {name} as {about:?}",
                parent.join(" ")
            )),
        }
    }
    assert!(unmarked.is_empty(), "{}", unmarked.join("\n"));
    // `adapters` at the root, its four commands and `connections repair`.
    assert_eq!(deprecated.len(), 6, "{deprecated:?}");
    for path in help_paths(&plan).into_iter().filter(|path| path.len() == 2) {
        let output = cli(&[path[0].as_str(), path[1].as_str(), "--help"]);
        let help = text(&output.stdout);
        for line in help.lines().filter(|line| line.contains("--adapter")) {
            assert!(
                line.trim_start()
                    .trim_start_matches("--adapter")
                    .contains("Deprecated"),
                "{path:?}: {line}"
            );
        }
    }
}

#[test]
fn help_on_a_deprecated_name_prints_no_deprecation_line() {
    let plan = connectors_cli_contract::plan();
    let mut cases: Vec<Vec<String>> = plan
        .commands
        .iter()
        .flat_map(|command| command.aliases.clone())
        .collect();
    cases.push(vec!["adapters".into()]);
    cases.push(
        ["instances", "describe", "--adapter", "forge"]
            .map(str::to_owned)
            .to_vec(),
    );
    cases.push(
        ["adapters", "status", "--adapter", "forge"]
            .map(str::to_owned)
            .to_vec(),
    );
    for output in ["human", "json"] {
        for case in &cases {
            let args: Vec<&str> = ["--output", output]
                .into_iter()
                .chain(case.iter().map(String::as_str))
                .chain(["--help"])
                .collect();
            let printed = cli(&args);
            assert_eq!(
                (printed.status.code(), text(&printed.stderr)),
                (Some(0), String::new()),
                "{args:?}"
            );
            assert!(text(&printed.stdout).contains("Usage:"), "{args:?}");
        }
    }
}

struct NoSources;
impl Sources for NoSources {
    fn acquire(&mut self, _: ProtectedSource) -> Result<String, AcquireError> {
        Err(AcquireError::Unavailable)
    }
}

struct Fails(Value);
impl Handler for Fails {
    fn call(&mut self, _: &Invocation<'_>) -> HandlerReply {
        HandlerReply::Error {
            code: "failure".into(),
            data: self.0.clone(),
        }
    }
}

/// The variants of the enum field `field` of the declared `failure` data.
fn variants(plan: &Plan, field: &str) -> Vec<String> {
    let Shape::Struct { fields } = &plan.callables["setup-check"].errors["failure"].shape else {
        panic!("failure data is a struct");
    };
    let Shape::Enum { variants } = &fields[field] else {
        panic!("{field} is an enum");
    };
    variants.clone()
}

#[test]
fn a_failure_rendered_for_a_person_never_says_adapter() {
    let plan = connectors_cli_contract::plan();
    let mut rendered = 0;
    for field in ["code", "stage", "next_action"] {
        let variants = variants(&plan, field);
        assert!(!variants.is_empty(), "{field}");
        for variant in variants {
            let mut data = json!({"kind":"operational","code":"invalid_input","stage":"arguments","next_action":"none"});
            data[field] = json!(variant);
            let output = connectors_cli_contract::run(
                ["connectors", "--output", "human", "setup", "check"]
                    .map(OsString::from)
                    .to_vec(),
                &mut NoSources,
                &mut Fails(data),
                None,
            );
            assert_eq!(output.exit_code, 1, "{field} {variant}: {output:?}");
            assert!(output.stderr.contains(&variant), "{output:?}");
            assert!(
                !output.stderr.to_lowercase().contains("adapter"),
                "{field} {variant}: {}",
                output.stderr
            );
            rendered += 1;
        }
    }
    // 35 codes, 11 stages and 15 next actions; `instance_ambiguous` and
    // `connection_unresolved` are the newest (story:connectors-do-short-invocation).
    assert_eq!(rendered, 61);
}
