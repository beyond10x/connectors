//! A job of `release-binaries.yml` that downloads artifacts names the ones it takes.
//!
//! The jobs `publish` and `image` both wait for `build` and then run side by side. The image
//! build uploads one build record per build as an artifact of the run
//! (`<owner>~<repository>~<id>.dockerbuild`). A download step without `pattern` takes every
//! artifact of the run, so `publish` got six files where it expected four whenever `image`
//! ended first: run 45358347732 of tag v0.77.1, 2026-11-24, both attempts.
//!
//! The musl archive is checked static before it is packed, and the checksum manifest names it.
use regex::Regex;
use serde_yaml_ng::Value;
use std::{fs, path::Path};

/// The two build records of run 45358347732.
const BUILD_RECORDS: [&str; 2] = [
    "beyond10x~connectors~N17U26.dockerbuild",
    "beyond10x~connectors~UTVTGK.dockerbuild",
];

fn workflow() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/release-binaries.yml");
    serde_yaml_ng::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// The `with.pattern` of each `actions/download-artifact` step: job, pattern (`None` when the
/// step has none).
fn download_patterns(workflow: &Value) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    for (id, job) in workflow.get("jobs").and_then(Value::as_mapping).unwrap() {
        let steps = job.get("steps").and_then(Value::as_sequence).unwrap();
        for step in steps {
            let uses = step.get("uses").and_then(Value::as_str).unwrap_or("");
            if !uses.starts_with("actions/download-artifact@") {
                continue;
            }
            let pattern = step
                .get("with")
                .and_then(|with| with.get("pattern"))
                .and_then(Value::as_str)
                .map(str::to_string);
            found.push((id.as_str().unwrap().to_string(), pattern));
        }
    }
    found
}

/// The targets of the build matrix, which are the names of the archive artifacts.
fn targets(workflow: &Value) -> Vec<String> {
    workflow["jobs"]["build"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|entry| entry["target"].as_str().unwrap().to_string())
        .collect()
}

/// A pattern of literal text and `*`, matched against a whole artifact name.
fn matches(pattern: &str, name: &str) -> bool {
    assert!(
        pattern
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.*".contains(c)),
        "the pattern {pattern:?} uses more than literal text and `*`; this test reads only those"
    );
    let expression = format!("^{}$", regex::escape(pattern).replace(r"\*", ".*"));
    Regex::new(&expression).unwrap().is_match(name)
}

#[test]
fn every_download_step_names_its_artifacts() {
    let workflow = workflow();
    let steps = download_patterns(&workflow);
    assert!(
        steps.iter().any(|(job, _)| job == "publish")
            && steps.iter().any(|(job, _)| job == "image"),
        "expected a download step in `publish` and in `image`: {steps:?}"
    );
    for (job, pattern) in &steps {
        let pattern = pattern.as_deref().unwrap_or_else(|| {
            panic!("the download step of job `{job}` has no `pattern`: it takes every artifact of the run")
        });
        for record in BUILD_RECORDS {
            assert!(
                !matches(pattern, record),
                "the download step of job `{job}` takes the build record {record} with {pattern:?}"
            );
        }
    }
}

#[test]
fn the_publish_job_takes_the_archive_of_each_target() {
    let workflow = workflow();
    let targets = targets(&workflow);
    assert_eq!(targets.len(), 5, "{targets:?}");
    let pattern = download_patterns(&workflow)
        .into_iter()
        .find(|(job, _)| job == "publish")
        .and_then(|(_, pattern)| pattern)
        .expect("the download step of job `publish` has a `pattern`");
    for target in &targets {
        assert!(
            matches(&pattern, target),
            "{pattern:?} of job `publish` leaves out the archive of {target}"
        );
    }
}

/// The `run` script of the step of `job` named `name`.
fn script<'a>(workflow: &'a Value, job: &str, name: &str) -> &'a str {
    workflow["jobs"][job]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .and_then(|step| step["run"].as_str())
        .unwrap_or_else(|| panic!("job `{job}` has no step {name:?} with a script"))
}

#[test]
fn the_musl_archive_is_checked_static_before_it_is_packed() {
    const MUSL: &str = "x86_64-unknown-linux-musl";
    let workflow = workflow();
    assert!(targets(&workflow).iter().any(|target| target == MUSL));
    let steps = workflow["jobs"]["build"]["steps"].as_sequence().unwrap();
    let position = |found: &dyn Fn(&Value) -> bool| steps.iter().position(found);
    let check = position(&|step| {
        step["if"].as_str().is_some_and(|guard| guard.contains(MUSL))
            && step["run"].as_str().is_some_and(|run| {
                run.contains("deploy/release/check-static.sh check") && run.contains("$BINARIES")
            })
    })
    .expect("no step of job `build` runs deploy/release/check-static.sh check over $BINARIES for the musl target");
    let build = position(&|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("cargo build"))
    })
    .expect("job `build` has a `cargo build` step");
    let package = position(&|step| step["name"].as_str() == Some("Package"))
        .expect("job `build` has a `Package` step");
    assert!(
        build < check && check < package,
        "the static check runs at step {check}, not between the build ({build}) and Package ({package})"
    );
}

#[test]
fn the_checksum_manifest_names_the_archive_of_each_target() {
    let workflow = workflow();
    let targets = targets(&workflow);
    let manifest = script(&workflow, "publish", "Create the checksum manifest");
    for target in &targets {
        assert!(
            manifest.contains(&format!(" {target}")),
            "the checksum manifest does not require the archive of {target}"
        );
    }
    assert!(
        manifest.contains(&format!("-eq {}", targets.len())),
        "the checksum manifest does not count {} archives",
        targets.len()
    );
}
