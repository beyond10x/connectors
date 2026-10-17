//! The gate (`gate.rs`, "Compile generated obligations and authored traces")
//! hands the authored scenarios to `ess verify conform synthesize --scenarios`,
//! which resolves their names and refuses an input-guarded outcome the model
//! cannot answer, but never executes a timeline. A scenario whose expected
//! outcome depends on the lifecycle state (`MarkConnectionStale` on a Stale or
//! Revoked connection, `PublishBinding` from Stale, `RevokeConnection` from
//! Stale) therefore compiles against a model that contradicts it.
//!
//! This test executes the authored scenarios of story
//! ess-connector-vocabulary-model against the model with the pinned `ess`
//! interpreter and requires every one of them to pass. Mutating the model's
//! `mark_stale` to `from: [Live, Stale]`, `publish` to `from: [Live]` or
//! `revoke` to `from: [Live]` leaves the gate green and fails this test.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const DIRECTORIES: &[&str] = &[
    "contracts/auth/connection/v1alpha1/scenarios",
    "contracts/service/v1alpha1/scenarios",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_connection_vocabulary_authored_scenarios_pass_when_executed() {
    let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
    let work = tempfile::tempdir().unwrap();
    let scenarios = work.path().join("scenarios");
    std::fs::create_dir(&scenarios).unwrap();
    let mut files = 0;
    for (index, directory) in DIRECTORIES.iter().enumerate() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(root().join(directory))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "yaml" || e == "yml"))
            .collect();
        entries.sort();
        for source in entries {
            let name = source.file_name().unwrap().to_string_lossy().into_owned();
            std::fs::copy(&source, scenarios.join(format!("{index}-{name}"))).unwrap();
            files += 1;
        }
    }
    assert!(
        files >= 5,
        "expected the story's five authored scenarios, found {files}"
    );

    let report = work.path().join("report.json");
    // The interpreter reports other domains' synthesized scenarios as
    // unsupported and so exits non-zero; the authored outcomes are read from
    // the report instead of the exit status.
    let output = Command::new(&ess)
        .current_dir(root())
        .env("TMPDIR", work.path())
        .args([
            "verify",
            "conform",
            "run",
            "--target",
            "interpreted",
            "--path",
        ])
        // A mutated copy of the model can be named to watch this test fail.
        .arg(std::env::var_os("CONNECTORS_ADVERSARY_MODEL").unwrap_or_else(|| "ess".into()))
        .arg("--scenarios")
        .arg(&scenarios)
        .arg("--report-out")
        .arg(&report)
        .args(["--report-format", "2"])
        .output()
        .expect("run ess");
    let report: Value = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|error| {
        panic!(
            "no report ({error}); ess said: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }))
    .unwrap();

    let mut passed = Vec::new();
    let mut other = Vec::new();
    for (status, names) in report["outcomes"].as_object().unwrap() {
        for name in names.as_array().unwrap() {
            let name = name.as_str().unwrap();
            if name.contains("/authored/") {
                if status == "passed" {
                    passed.push(name.to_owned());
                } else {
                    other.push(format!("{status}: {name}"));
                }
            }
        }
    }
    assert!(
        other.is_empty(),
        "authored scenarios not passing: {other:#?}"
    );
    assert_eq!(
        passed.len(),
        files,
        "every authored scenario file executes and passes: {passed:#?}"
    );
}
