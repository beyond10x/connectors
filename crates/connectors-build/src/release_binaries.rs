//! Release binaries: every binary an authored guide runs as an installed command is one
//! the release workflow packages.
//!
//! The packaged binaries are the build job's `BINARIES` in the release workflow; its
//! `cargo build` step and its `Package` step must both iterate that list, so the list is
//! what the archives hold. A guide is `README.md`, a Markdown file directly under `docs/`,
//! or an authored page under `website/docs/` (the generated `reference/` excluded). A guide
//! runs an installed binary where, inside a fenced `sh`, `shell`, `bash`, `zsh` or `console`
//! block, a workspace binary's bare name is the first word of a command (after variable
//! assignments and `sudo`, `env`, `exec`, `nohup`, `time`, `command`), or where a path
//! into an extracted release archive (`connectors-$version-$target/<name>`) names it. A
//! path such as `target/release/<name>` runs a binary built from source and is not one.
use crate::Result;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workflow that builds the release archives.
pub const WORKFLOW: &str = ".github/workflows/release-binaries.yml";

/// One place a guide runs a binary the release archives do not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub guide: String,
    pub line: usize,
    pub binary: String,
}

/// Check this repository: the workflow's list names workspace binaries, and no guide runs
/// an installed binary outside it.
pub fn run(root: &Path) -> Result<()> {
    let workflow = std::fs::read_to_string(root.join(WORKFLOW))
        .map_err(|error| format!("cannot read {WORKFLOW}: {error}"))?;
    let packaged = packaged(&workflow)?;
    let binaries = workspace_binaries(root)?;
    known(&packaged, &binaries)?;
    let guides = guides(root)?;
    let mut findings = Vec::new();
    for path in &guides {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let name = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        findings.extend(scan(&name, &text, &packaged, &binaries));
    }
    if !findings.is_empty() {
        let lines: Vec<String> = findings
            .iter()
            .map(|f| format!("{}:{}: runs `{}`", f.guide, f.line, f.binary))
            .collect();
        return Err(format!(
            "guides run installed binaries {WORKFLOW} does not package (it packages {}):\n{}",
            packaged.join(", "),
            lines.join("\n")
        )
        .into());
    }
    println!(
        "release-binaries: {} guides run only packaged binaries ({}); exit=0",
        guides.len(),
        packaged.join(", ")
    );
    Ok(())
}

/// The binaries the release archives hold: the build job's `BINARIES`, which its
/// `cargo build` step and its `Package` step must both iterate.
pub fn packaged(workflow: &str) -> Result<Vec<String>> {
    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(workflow)
        .map_err(|error| format!("cannot parse the release workflow: {error}"))?;
    let build = &document["jobs"]["build"];
    let list = build["env"]["BINARIES"]
        .as_str()
        .ok_or("the release workflow's build job sets no BINARIES list")?;
    let names: Vec<String> = list.split_whitespace().map(String::from).collect();
    if names.is_empty() {
        return Err("the release workflow's BINARIES list is empty".into());
    }
    let steps = build["steps"]
        .as_sequence()
        .ok_or("the release workflow's build job has no steps")?;
    let scripts: Vec<(&str, &str)> = steps
        .iter()
        .filter_map(|step| Some((step["name"].as_str()?, step["run"].as_str()?)))
        .collect();
    let iterates = |script: &str| script.contains("$BINARIES") || script.contains("${BINARIES}");
    let build_step = scripts
        .iter()
        .find(|(_, script)| script.contains("cargo build"))
        .ok_or("the release workflow's build job has no `cargo build` step")?;
    if !iterates(build_step.1) {
        return Err(format!(
            "the release workflow's `cargo build` step ({}) does not iterate BINARIES",
            build_step.0
        )
        .into());
    }
    let package = scripts
        .iter()
        .find(|(name, _)| *name == "Package")
        .ok_or("the release workflow's build job has no `Package` step")?;
    if !iterates(package.1) {
        return Err("the release workflow's `Package` step does not iterate BINARIES".into());
    }
    Ok(names)
}

/// Every packaged name is a binary target of the workspace.
pub fn known(packaged: &[String], binaries: &BTreeSet<String>) -> Result<()> {
    let unknown: Vec<&str> = packaged
        .iter()
        .filter(|name| !binaries.contains(*name))
        .map(String::as_str)
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the release workflow packages names that are no workspace binary: {}",
            unknown.join(", ")
        )
        .into())
    }
}

/// Every place `text` runs an installed workspace binary that `packaged` leaves out.
pub fn scan(
    guide: &str,
    text: &str,
    packaged: &[String],
    binaries: &BTreeSet<String>,
) -> Vec<Finding> {
    let archive = Regex::new(
        r"connectors-(?:\$\{?version\}?|[0-9]+\.[0-9]+\.[0-9]+)-(?:\$\{?target\}?|[A-Za-z0-9_]+(?:-[A-Za-z0-9_]+)*)/([A-Za-z0-9_.-]+)",
    )
    .expect("archive path pattern");
    let line_of = |offset: usize| text[..offset].matches('\n').count() + 1;
    let mut findings = Vec::new();
    let mut shell: Option<bool> = None;
    let mut continued = false;
    let report = |line: usize, name: &str, findings: &mut Vec<Finding>| {
        if binaries.contains(name) && !packaged.iter().any(|p| p == name) {
            findings.push(Finding {
                guide: guide.to_string(),
                line,
                binary: name.to_string(),
            });
        }
    };
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let language = info.split_whitespace().next().unwrap_or("");
                shell = match language {
                    "sh" | "shell" | "bash" | "zsh" => Some(false),
                    "console" => Some(true),
                    _ => None,
                };
                continued = false;
            }
            Event::End(TagEnd::CodeBlock) => shell = None,
            Event::Text(content) => {
                let Some(console) = shell else { continue };
                let first = line_of(range.start);
                for (index, raw) in content.lines().enumerate() {
                    let line = first + index;
                    let mut command = raw.trim();
                    if console {
                        match command.strip_prefix('$') {
                            Some(rest) => command = rest.trim_start(),
                            None => {
                                continued = false;
                                continue;
                            }
                        }
                    }
                    for capture in archive.captures_iter(command) {
                        report(line, &capture[1], &mut findings);
                    }
                    let starts = !continued && !command.starts_with('#');
                    continued = command.ends_with('\\');
                    if !starts {
                        continue;
                    }
                    for program in programs(command) {
                        report(line, program, &mut findings);
                    }
                }
            }
            _ => {}
        }
    }
    findings
}

/// The first word of each command on a shell line.
fn programs(line: &str) -> Vec<&str> {
    const PREFIXES: [&str; 13] = [
        "sudo", "env", "exec", "nohup", "time", "command", "!", "do", "then", "else", "if",
        "while", "until",
    ];
    let separators = ["&&", "||", "|", ";", "$(", "(", "`", "&", "{"];
    let mut segments = vec![line];
    for separator in separators {
        segments = segments
            .into_iter()
            .flat_map(|segment| segment.split(separator))
            .collect();
    }
    segments
        .into_iter()
        .filter_map(|segment| {
            segment
                .split_whitespace()
                .find(|word| !PREFIXES.contains(word) && !assignment(word))
                .map(|word| word.trim_matches(|c| c == '"' || c == '\''))
        })
        .collect()
}

fn assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit())
    })
}

/// `README.md`, Markdown directly under `docs/`, and the authored pages under
/// `website/docs/` (the generated `reference/` excluded), sorted.
pub fn guides(root: &Path) -> Result<Vec<PathBuf>> {
    let mut guides = vec![root.join("README.md")];
    for entry in std::fs::read_dir(root.join("docs"))? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            guides.push(path);
        }
    }
    let website = root.join("website/docs");
    let mut pending = vec![website.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                if path != website.join("reference") {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "md" || e == "mdx") {
                guides.push(path);
            }
        }
    }
    guides.sort();
    Ok(guides)
}

/// The names of every binary target of the workspace's packages.
fn workspace_binaries(root: &Path) -> Result<BTreeSet<String>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = super::run(Command::new(cargo).current_dir(root).args([
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
        "--locked",
        "--offline",
    ]))?;
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut binaries = BTreeSet::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or("cargo metadata lists no packages")?
    {
        for target in package["targets"].as_array().into_iter().flatten() {
            let bin = target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"));
            if bin && let Some(name) = target["name"].as_str() {
                binaries.insert(name.to_string());
            }
        }
    }
    Ok(binaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/release-binaries")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn binaries() -> BTreeSet<String> {
        [
            "connectors",
            "connectors-catalog-provider",
            "connectors-grafana",
            "connectors-kubernetes",
            "connectors-loki",
            "connectors-prometheus",
            "connectors-sql",
            "connectors-tavily",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn the_packaged_binaries_are_the_build_jobs_list() {
        assert_eq!(
            packaged(&fixture("workflow.yml")).unwrap(),
            strings(&["connectors", "connectors-catalog-provider"])
        );
    }

    #[test]
    fn a_guide_running_an_unpackaged_binary_is_refused_at_each_place() {
        let packaged = packaged(&fixture("workflow.yml")).unwrap();
        let found = scan(
            "guide-unpackaged.md",
            &fixture("guide-unpackaged.md"),
            &packaged,
            &binaries(),
        );
        let found: Vec<(usize, &str)> = found.iter().map(|f| (f.line, f.binary.as_str())).collect();
        assert_eq!(
            found,
            [
                (6, "connectors-sql"),
                (7, "connectors-kubernetes"),
                (10, "connectors-grafana"),
                (18, "connectors-prometheus"),
            ]
        );
    }

    #[test]
    fn a_guide_running_only_packaged_binaries_passes() {
        let packaged = packaged(&fixture("workflow.yml")).unwrap();
        assert_eq!(
            scan(
                "guide-packaged.md",
                &fixture("guide-packaged.md"),
                &packaged,
                &binaries()
            ),
            []
        );
    }

    #[test]
    fn a_workflow_without_a_binaries_list_is_refused() {
        let workflow = fixture("workflow.yml").replace(
            "      BINARIES: connectors connectors-catalog-provider\n",
            "",
        );
        let error = packaged(&workflow).unwrap_err().to_string();
        assert!(error.contains("BINARIES"), "{error}");
    }

    #[test]
    fn a_workflow_whose_package_step_ignores_the_list_is_refused() {
        let workflow = fixture("workflow.yml").replace(
            "for binary in $BINARIES; do cp",
            "for binary in connectors; do cp",
        );
        let error = packaged(&workflow).unwrap_err().to_string();
        assert!(error.contains("Package"), "{error}");
    }

    #[test]
    fn a_workflow_whose_build_step_ignores_the_list_is_refused() {
        let workflow = fixture("workflow.yml").replace(
            "for binary in $BINARIES; do packages+=",
            "for binary in connectors; do packages+=",
        );
        let error = packaged(&workflow).unwrap_err().to_string();
        assert!(error.contains("cargo build"), "{error}");
    }

    #[test]
    fn a_packaged_name_that_is_no_workspace_binary_is_refused() {
        let error = known(&strings(&["connectors", "connectors-nope"]), &binaries())
            .unwrap_err()
            .to_string();
        assert!(error.contains("connectors-nope"), "{error}");
        known(&strings(&["connectors"]), &binaries()).unwrap();
    }

    #[test]
    fn the_release_workflow_packages_the_cli_and_the_catalog_provider() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let workflow = std::fs::read_to_string(root.join(WORKFLOW)).unwrap();
        assert_eq!(
            packaged(&workflow).unwrap(),
            strings(&["connectors", "connectors-catalog-provider"])
        );
    }
}
