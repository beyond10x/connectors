//! A job of `release-binaries.yml` that downloads artifacts names the ones it takes.
//!
//! The jobs `publish` and `image` both wait for `build` and then run side by side. The image
//! build uploads one build record per build as an artifact of the run
//! (`<owner>~<repository>~<id>.dockerbuild`). A download step without `pattern` takes every
//! artifact of the run, so `publish` got six files where it expects four whenever `image`
//! ended first: run 45358347732 of tag v0.77.1, 2026-11-24, both attempts.
use regex::Regex;
use serde_yaml_ng::Value;
use std::{fs, path::Path};

/// The two build records of run 45358347732.
const BUILD_RECORDS: [&str; 2] = [
    "beyond10x~connectors~N17U26.dockerbuild",
    "beyond10x~connectors~UTVTGK.dockerbuild",
];

fn workflow() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github/workflows/release-binaries.yml");
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
        steps.iter().any(|(job, _)| job == "publish") && steps.iter().any(|(job, _)| job == "image"),
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
    assert_eq!(targets.len(), 4, "{targets:?}");
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
