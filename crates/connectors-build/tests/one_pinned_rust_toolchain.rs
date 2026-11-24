//! `rust-toolchain.toml` names the one Rust toolchain, and every workflow installs it from that
//! file (`story:one-pinned-rust-toolchain-for-the-local-gate-and-the-forge`). A workflow that
//! names a Rust version or a channel selects another compiler than the local gate reads, and a
//! release build of a tag from before the file is refused instead of built with a fallback.
use regex::Regex;
use serde_yaml_ng::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

const PIN: &str = "rust-toolchain.toml";

fn workflows_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows")
}

/// The simple commands of a step's script, each as its words without quotes: lines joined at a
/// trailing backslash, split at the shell's separators, comment lines left out.
fn commands(script: &str) -> Vec<Vec<String>> {
    let mut found = Vec::new();
    for line in script.replace("\\\n", " ").lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let mut separated = line.to_string();
        for separator in ["&&", "||", ";", "|", "(", ")", "`", "{", "}"] {
            separated = separated.replace(separator, "\n");
        }
        for command in separated.lines() {
            let words: Vec<String> = command
                .split_whitespace()
                .map(|word| word.trim_matches(['"', '\'', '$']).to_string())
                .filter(|word| !word.is_empty())
                .collect();
            if !words.is_empty() {
                found.push(words);
            }
        }
    }
    found
}

/// A Rust version or a channel name, alone, with a date or host suffix, or as `+<toolchain>`.
fn names_a_toolchain(word: &str) -> bool {
    Regex::new(r"^\+?(\d+\.\d+(\.\d+)?|stable|beta|nightly)(-[A-Za-z0-9_.-]+)?$")
        .unwrap()
        .is_match(word)
}

/// The toolchain argument of `rustup toolchain install` (or its alias `rustup install`): `None`
/// when the words are not that command, `Some(None)` when it has no such argument and so
/// installs the toolchain the file names.
fn install_argument(arguments: &[String]) -> Option<Option<&str>> {
    let rest = match arguments {
        [toolchain, install, rest @ ..] if toolchain == "toolchain" && install == "install" => rest,
        [install, rest @ ..] if install == "install" => rest,
        _ => return None,
    };
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        if ["--profile", "--component", "-c", "--target", "-t"].contains(&word.as_str()) {
            words.next();
        } else if !word.starts_with('-') {
            return Some(Some(word));
        }
    }
    Some(None)
}

#[derive(Default)]
struct Reading {
    violations: Vec<String>,
    /// Jobs with a step that runs `cargo` or `rustc`.
    rust_jobs: usize,
}

/// What one workflow file does against the rule: no Rust version or channel name in a command
/// or a step name, and no `cargo` or `rustc` before the job installed the toolchain of the file
/// from its checkout.
fn read(file: &str, text: &str) -> Reading {
    let mut reading = Reading::default();
    if text.contains("RUSTUP_TOOLCHAIN") {
        reading.violations.push(format!(
            "{file}: RUSTUP_TOOLCHAIN selects a toolchain; {PIN} names it"
        ));
    }
    let workflow: Value =
        serde_yaml_ng::from_str(text).unwrap_or_else(|error| panic!("{file}: {error}"));
    let jobs = workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("{file}: no jobs"));
    let version = Regex::new(r"\d+\.\d+|\b(stable|beta|nightly)\b").unwrap();
    for (id, job) in jobs {
        let id = id.as_str().unwrap_or("?");
        // A job without steps calls another workflow.
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };
        let (mut checked_out, mut installed, mut runs_rust) = (false, false, false);
        for (index, step) in steps.iter().enumerate() {
            let name = step.get("name").and_then(Value::as_str);
            let at = match name {
                Some(name) => format!("{file}: job {id}: step \"{name}\""),
                None => format!("{file}: job {id}: step {}", index + 1),
            };
            let uses = step.get("uses").and_then(Value::as_str).unwrap_or("");
            if uses.starts_with("actions/checkout@")
                && step.get("with").and_then(|with| with.get("path")).is_none()
            {
                checked_out = true;
            }
            if uses.to_ascii_lowercase().contains("toolchain") {
                reading.violations.push(format!(
                    "{at}: the action {uses} installs a toolchain it names; {PIN} names it"
                ));
            }
            let commands = commands(step.get("run").and_then(Value::as_str).unwrap_or(""));
            let calls_rustup = commands
                .iter()
                .any(|command| command.iter().any(|word| word == "rustup"));
            if let Some(name) = name {
                let about_rust = calls_rustup || name.to_ascii_lowercase().contains("rust");
                if about_rust && version.is_match(name) {
                    reading
                        .violations
                        .push(format!("{at}: the step name holds a Rust version"));
                }
            }
            let mut before_install = false;
            for command in &commands {
                let shown = command.join(" ");
                if let Some(rustup) = command.iter().position(|word| word == "rustup") {
                    let arguments = &command[rustup + 1..];
                    if let Some(named) = arguments.iter().find(|word| names_a_toolchain(word)) {
                        reading.violations.push(format!(
                            "{at}: `{shown}` names the toolchain {named}; {PIN} names it"
                        ));
                    } else {
                        match install_argument(arguments) {
                            None => {}
                            Some(Some(named)) => reading.violations.push(format!(
                                "{at}: `{shown}` installs {named}, not the toolchain of {PIN}"
                            )),
                            Some(None) if step.get("working-directory").is_some() => {
                                reading.violations.push(format!(
                                    "{at}: `{shown}` runs in another directory than the checkout, where no {PIN} is read"
                                ));
                            }
                            Some(None) if !checked_out => reading.violations.push(format!(
                                "{at}: `{shown}` runs before the checkout that holds {PIN}"
                            )),
                            Some(None) => installed = true,
                        }
                    }
                }
                for (position, word) in command.iter().enumerate() {
                    if word != "cargo" && word != "rustc" {
                        continue;
                    }
                    runs_rust = true;
                    before_install |= !installed;
                    let selector = command
                        .get(position + 1)
                        .filter(|word| word.starts_with('+') && names_a_toolchain(word));
                    if let Some(selector) = selector {
                        reading.violations.push(format!(
                            "{at}: `{shown}` names the toolchain {selector}; {PIN} names it"
                        ));
                    }
                }
            }
            if before_install {
                reading.violations.push(format!(
                    "{at}: runs cargo or rustc with no earlier `rustup toolchain install` of {PIN} in the job"
                ));
            }
        }
        reading.rust_jobs += usize::from(runs_rust);
    }
    reading
}

/// A workflow that keeps the rule; each failing copy below differs from it in one place.
const KEEPS_THE_RULE: &str = r#"name: copy
on: pull_request
jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v7
      - name: Install the Rust toolchain of rust-toolchain.toml
        run: |
          rustup toolchain install
          rustup target add "$TARGET"
          rustup default "$(rustup show active-toolchain | head -n 1 | cut -d' ' -f1)"
          rustc --version
      - name: Build
        run: cargo build --locked
"#;

const INSTALL_STEP: &str = r#"      - name: Install the Rust toolchain of rust-toolchain.toml
        run: |
          rustup toolchain install
          rustup target add "$TARGET"
          rustup default "$(rustup show active-toolchain | head -n 1 | cut -d' ' -f1)"
          rustc --version
"#;

/// `KEEPS_THE_RULE` with one text replaced by another; the text must be there.
fn copy(from: &str, to: &str) -> String {
    assert!(KEEPS_THE_RULE.contains(from), "{from:?} is not in the copy");
    KEEPS_THE_RULE.replacen(from, to, 1)
}

#[test]
fn no_workflow_names_a_rust_version() {
    let mut files: Vec<PathBuf> = fs::read_dir(workflows_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    let (mut violations, mut rust_jobs) = (Vec::new(), 0);
    for path in &files {
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let reading = read(&file, &fs::read_to_string(path).unwrap());
        violations.extend(reading.violations);
        rust_jobs += reading.rust_jobs;
    }
    assert!(
        violations.is_empty(),
        "{} places in .github/workflows name a Rust toolchain or run Rust without the one of {PIN}:\n{}",
        violations.len(),
        violations.join("\n")
    );
    assert!(
        rust_jobs > 0,
        "no job of {} workflow files runs cargo; the rule read nothing",
        files.len()
    );
    assert!(
        workflows_directory().join("../..").join(PIN).is_file(),
        "the workflows install the toolchain of {PIN}, and the repository root has no such file"
    );

    assert_eq!(
        read("copy", KEEPS_THE_RULE).violations,
        Vec::<String>::new()
    );
    let install = "          rustup toolchain install\n";
    let failing = [
        (
            "a version as the default",
            copy(
                install,
                "          rustup toolchain install\n          rustup default 1.98.1\n",
            ),
            "`rustup default 1.98.1` names the toolchain 1.98.1",
        ),
        (
            "a channel name as the default",
            copy(
                install,
                "          rustup toolchain install\n          rustup default stable\n",
            ),
            "`rustup default stable` names the toolchain stable",
        ),
        (
            "cargo with no install step",
            copy(INSTALL_STEP, ""),
            "step \"Build\": runs cargo or rustc with no earlier",
        ),
        (
            "a version in the install command",
            copy(
                install,
                "          rustup toolchain install 1.98.1 --profile minimal\n",
            ),
            "names the toolchain 1.98.1",
        ),
        (
            "a toolchain from a variable",
            copy(install, "          rustup toolchain install \"$RUST\"\n"),
            "installs RUST, not the toolchain of rust-toolchain.toml",
        ),
        (
            "a version in the step name",
            copy(
                "Install the Rust toolchain of rust-toolchain.toml",
                "Install Rust 1.98.1",
            ),
            "the step name holds a Rust version",
        ),
        (
            "a toolchain selector on cargo",
            copy("cargo build --locked", "cargo +nightly build --locked"),
            "names the toolchain +nightly",
        ),
        (
            "the install before the checkout",
            copy("      - uses: actions/checkout@v7\n", ""),
            "runs before the checkout that holds rust-toolchain.toml",
        ),
        (
            "a checkout into a subdirectory",
            copy(
                "      - uses: actions/checkout@v7\n",
                "      - uses: actions/checkout@v7\n        with:\n          path: repository\n",
            ),
            "runs before the checkout that holds rust-toolchain.toml",
        ),
        (
            "the install in another directory",
            copy(
                "        run: |\n          rustup toolchain install\n",
                "        working-directory: ${{ runner.temp }}\n        run: |\n          rustup toolchain install\n",
            ),
            "runs in another directory than the checkout",
        ),
        (
            "an action that installs a named toolchain",
            copy(
                "      - uses: actions/checkout@v7\n",
                "      - uses: actions/checkout@v7\n      - uses: dtolnay/rust-toolchain@stable\n",
            ),
            "the action dtolnay/rust-toolchain@stable installs a toolchain it names",
        ),
        (
            "the toolchain in the environment",
            copy(
                "    runs-on: ubuntu-24.04\n",
                "    runs-on: ubuntu-24.04\n    env:\n      RUSTUP_TOOLCHAIN: \"1.98.1\"\n",
            ),
            "RUSTUP_TOOLCHAIN selects a toolchain",
        ),
    ];
    for (case, text, expected) in failing {
        let violations = read("copy", &text).violations;
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(expected)),
            "{case}: the rule did not refuse this copy with {expected:?}; it found {violations:?}\n{text}"
        );
    }
}

/// Runs each install step of the release workflow as the runner does, with stubs in place of
/// `rustup` and `rustc`.
#[cfg(unix)]
mod release {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        process::{Command, Output},
    };

    const TAG: &str = "v9.87.65";

    /// Each step of `release-binaries.yml` whose script calls `rustup`: job, script, and the
    /// expression its `TAG` is set from.
    fn install_steps() -> Vec<(String, String, String)> {
        let text = fs::read_to_string(workflows_directory().join("release-binaries.yml")).unwrap();
        let workflow: Value = serde_yaml_ng::from_str(&text).unwrap();
        let mut found = Vec::new();
        for (id, job) in workflow.get("jobs").and_then(Value::as_mapping).unwrap() {
            let id = id.as_str().unwrap();
            let steps = job.get("steps").and_then(Value::as_sequence).unwrap();
            for step in steps {
                let script = step.get("run").and_then(Value::as_str).unwrap_or("");
                if !commands(script)
                    .iter()
                    .any(|command| command.iter().any(|word| word == "rustup"))
                {
                    continue;
                }
                let tag = step
                    .get("env")
                    .and_then(|env| env.get("TAG"))
                    .or_else(|| job.get("env").and_then(|env| env.get("TAG")))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                found.push((id.to_string(), script.to_string(), tag.to_string()));
            }
        }
        found
    }

    fn stub(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// The script run with `sh -e` in `checkout`; the calls of the `rustup` stub, one per line.
    fn run(script: &str, root: &Path, checkout: &Path) -> (Output, String) {
        let bin = root.join("bin");
        let calls = root.join("rustup-calls");
        fs::create_dir_all(&bin).unwrap();
        let _ = fs::remove_file(&calls);
        stub(
            &bin.join("rustup"),
            "echo \"$*\" >> \"$RUSTUP_CALLS\"\necho 0.0.0-stub-host",
        );
        stub(&bin.join("rustc"), "echo \"rustc stub\"");
        fs::write(root.join("step.sh"), script).unwrap();
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let output = Command::new("sh")
            .arg("-e")
            .arg(root.join("step.sh"))
            .current_dir(checkout)
            .env("PATH", path)
            .env("RUSTUP_CALLS", &calls)
            .env("TAG", TAG)
            .env("TARGET", "stub-target")
            .output()
            .unwrap();
        (output, fs::read_to_string(&calls).unwrap_or_default())
    }

    #[test]
    fn a_release_build_of_a_tag_without_the_pin_is_refused() {
        let steps = install_steps();
        let jobs: Vec<&str> = steps.iter().map(|(job, ..)| job.as_str()).collect();
        assert_eq!(jobs, ["provenance", "build"], "the jobs that install Rust");
        for (job, script, tag) in &steps {
            assert!(
                !script.contains("${{"),
                "job {job}: the install script holds an expression the runner replaces, so `sh` cannot run it as written:\n{script}"
            );
            let root = tempfile::tempdir().unwrap();

            let empty = root.path().join("tag-without-the-pin");
            fs::create_dir(&empty).unwrap();
            let (output, calls) = run(script, root.path(), &empty);
            let said = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                !output.status.success(),
                "job {job}: the install step ended 0 in a checkout without {PIN}:\n{said}"
            );
            assert!(
                said.contains(PIN) && said.contains(TAG),
                "job {job}: the refusal names neither {PIN} nor the tag {TAG}:\n{said}"
            );
            assert_eq!(calls, "", "job {job}: rustup was called before the refusal");
            assert_eq!(
                fs::read_dir(&empty).unwrap().count(),
                0,
                "job {job}: the refused step wrote into the checkout"
            );
            assert!(
                tag.contains("inputs.tag") || tag.contains("needs.provenance.outputs.tag"),
                "job {job}: the install step reads TAG, which must be set from the workflow's tag: {tag:?}"
            );

            // The same step and stubs in a checkout that holds the file: it installs from it.
            let pinned = root.path().join("tag-with-the-pin");
            fs::create_dir(&pinned).unwrap();
            fs::write(pinned.join(PIN), "").unwrap();
            let (output, calls) = run(script, root.path(), &pinned);
            assert!(
                output.status.success(),
                "job {job}: the install step failed in a checkout with {PIN}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                calls.lines().next(),
                Some("toolchain install"),
                "job {job}: the first rustup call with {PIN} present"
            );
        }
    }
}
