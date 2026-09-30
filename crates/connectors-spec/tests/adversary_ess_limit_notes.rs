//! Adversary, story:ess-pin-newest-release. An ESS-LIMIT note is a claim about the ESS release the
//! repository pins. This drives the pinned `ess` against one such claim.
//!
//! `ess/domains/execution_audit.yaml` (ESS-LIMIT) says the pinned ESS synthesizes
//! `AppendFinalObservation/recorded` under a `when:` guard
//! `observation_id == final_observation.observation_id`, and that the guard is left undeclared
//! for a host reason, not an ESS one.
use std::{path::Path, process::Command};

const NOTE: &str = "#   ESS 0.45 synthesizes `recorded` under that `when:` guard and the Entity Runtime\n#   lowering carries it.";
const RECORDED: &str = "      - name: recorded\n        when: decision == allow\n";
const GUARDED: &str = "      - name: recorded\n        when:\n          all:\n            - decision == allow\n            - observation_id == final_observation.observation_id\n";
const SCENARIO: &str = "connectors.execution_audit.AppendFinalObservation/outcome/recorded";

#[test]
fn the_observation_id_equality_note_holds_for_the_pinned_ess() {
    let ess = connectors_spec::toolchain::resolve(None).unwrap();
    let pinned = connectors_spec::toolchain::pin().unwrap().ess;
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ess");
    let audit = std::fs::read_to_string(source.join("domains/execution_audit.yaml")).unwrap();
    assert!(audit.contains(NOTE), "the note under test is gone");
    assert!(audit.contains(RECORDED), "the branch under test changed");

    let work = tempfile::tempdir().unwrap();
    let spec = work.path().join("ess");
    std::fs::create_dir_all(spec.join("domains")).unwrap();
    for entry in std::fs::read_dir(&source).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            std::fs::copy(&path, spec.join(path.file_name().unwrap())).unwrap();
        }
    }
    for entry in std::fs::read_dir(source.join("domains")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let text = if name == "execution_audit.yaml" {
            text.replacen(RECORDED, GUARDED, 1)
        } else {
            text
        };
        std::fs::write(spec.join("domains").join(name), text).unwrap();
    }

    let suite = work.path().join("suite.json");
    let output = Command::new(&ess)
        .args(["verify", "conform", "synthesize", "--path"])
        .arg(&spec)
        .args([
            "--component",
            "local-metadata-authority",
            "--target",
            "ir",
            "--out",
        ])
        .arg(&suite)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !said.contains(&format!("has no scenario `{SCENARIO}`")),
        "ESS {pinned} refuses `AppendFinalObservation/recorded` under the guard \
         `observation_id == final_observation.observation_id`, so the ESS-LIMIT note in \
         ess/domains/execution_audit.yaml (\"ESS 0.45 synthesizes `recorded` under that `when:` \
         guard\") is false for the pinned release:\n{said}"
    );
    let suite: serde_json::Value = serde_json::from_slice(&std::fs::read(&suite).unwrap()).unwrap();
    assert!(
        suite["scenarios"].get(SCENARIO).is_some(),
        "ESS {pinned} wrote no `{SCENARIO}` scenario under the guard"
    );
}
