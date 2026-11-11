//! Adversary (story:test-binary-per-package): what building every catalog suite
//! into one test binary changes. A name handed to `--exact` that the binary does
//! not list runs 0 tests and exits 0, and a suite that sets process state now
//! shares the process with every other suite instead of having one to itself.
#![cfg(unix)]

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

fn sources(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The string literal at the start of `text`, after whitespace and commas.
fn literal(text: &str) -> Option<&str> {
    let text = text.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
    let rest = text.strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

/// A libtest name: `::`-separated lowercase identifiers.
fn is_test_path(name: &str) -> bool {
    !name.is_empty()
        && name.split("::").all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// Each `(file, name)` the suites hand to `--exact` when they run this binary
/// again: a literal after `"--exact"`, a `const …NAME: &str` holding a module
/// path (the `reenter` callers) and the argument of adversary_git_relay's
/// `check("…")`.
fn rerun_names() -> BTreeSet<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    sources(&root, &mut files);
    let mut names = BTreeSet::new();
    for file in files {
        if file.ends_with("adversary_one_binary.rs") {
            continue;
        }
        let text = fs::read_to_string(&file).unwrap();
        let shown = file.strip_prefix(&root).unwrap().display().to_string();
        let mut add = |name: &str| {
            if is_test_path(name) {
                names.insert((shown.clone(), name.to_owned()));
            }
        };
        for (at, flag) in text.match_indices("\"--exact\"") {
            if let Some(name) = literal(&text[at + flag.len()..]) {
                add(name);
            }
        }
        for line in text.lines() {
            let line = line.trim_start();
            let Some(rest) = line
                .strip_prefix("const ")
                .or_else(|| line.strip_prefix("pub(super) const "))
            else {
                continue;
            };
            let Some((ident, value)) = rest.split_once(": &str =") else {
                continue;
            };
            if ident.ends_with("NAME")
                && let Some(name) = literal(value)
                && name.contains("::")
            {
                add(name);
            }
        }
        for (at, call) in text.match_indices("check(\"") {
            let before = text[..at].chars().next_back();
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            if let Some(name) = literal(&text[at + call.len() - 1..]) {
                add(name);
            }
        }
    }
    names
}

/// Every test this binary lists, ignored ones included.
fn listed() -> BTreeSet<String> {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--list", "--format", "terse"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

/// A re-run by a name the binary does not list selects nothing and exits 0,
/// so each such name must be one `--list` prints.
#[test]
fn every_name_a_suite_runs_this_binary_again_with_is_a_test_it_lists() {
    let names = rerun_names();
    assert!(
        names.len() >= 11,
        "fewer re-run sites than the sources hold: {names:#?}"
    );
    for site in [
        (
            "adversary_git_relay.rs",
            "adversary_git_relay::fixture_scan_check",
        ),
        ("git_relay.rs", "git_relay::pre_push_check_fixture"),
        (
            "local_runtime/owner_replay.rs",
            "local_runtime::cli_journey::guarded_merge::owner_replay::catalog_same_image_owner_fixture",
        ),
    ] {
        assert!(
            names.contains(&(site.0.to_owned(), site.1.to_owned())),
            "the scan missed {site:?}: {names:#?}"
        );
    }
    let listed = listed();
    assert!(listed.len() > 1000, "{} tests listed", listed.len());
    let missing: Vec<_> = names
        .iter()
        .filter(|(_, name)| !listed.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "re-run names this binary does not list: {missing:#?}"
    );
}

/// derived_instance_adversary.rs sets `CONNECTORS_INSTALLATION` for the whole
/// process while it loads a configuration (provider handshake included). Beside
/// the other suites that reached them and the children they start without
/// `env_clear`, so it stays its own `[[test]]` target and so its own process.
/// This case is red when a module of `main` sets the variable, as it was with
/// that file declared here and run in the gate's parallel `cargo test --workspace`.
#[test]
fn no_suite_sees_the_installation_another_suite_sets() {
    let initial = std::env::var_os("CONNECTORS_INSTALLATION");
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        let now = std::env::var_os("CONNECTORS_INSTALLATION");
        assert_eq!(
            now, initial,
            "another suite's CONNECTORS_INSTALLATION reached this one"
        );
        std::thread::sleep(Duration::from_micros(200));
    }
}
