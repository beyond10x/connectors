// Adversary cases for story:configuration-refusal-names-the-entry.
use connectors_host::local::{
    Failure,
    config::{Config, Paths, Refusal},
};
use std::fs;

fn paths(root: &tempfile::TempDir) -> Paths {
    Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap()
}

fn entry(alias: &str, instance: &str, private: &str, sha: &str) -> String {
    format!(
        "\n[adapters.{alias}]\ninstance_id='{instance}'\nadapter_id='fixture'\nconfiguration_revision='one'\nprotocol='v1alpha1'\n{private}[adapters.{alias}.executable]\npath='/fixture'\nsha256='{sha}'\n"
    )
}

// semantics.md: fields are named "only after the entry is otherwise admissible"
// and scenarios.md T04: "Any other invalid entry omits both fields". A file
// whose second entry is invalid for another reason is still refused naming the
// first entry, and whether it is depends only on how the aliases sort.
#[test]
fn a_file_invalid_for_another_reason_names_no_mismatched_entry() {
    let root = tempfile::tempdir().unwrap();
    let paths = paths(&root);
    Config::initialize(&paths).unwrap();
    let setup = fs::read_to_string(&paths.config)
        .unwrap()
        .replace("connectors-local/2", "connectors-local/1");
    let good = "a".repeat(64);
    let mismatched = entry(
        "alpha",
        "alpha-one",
        "private_protocol='connectors-private/2'\n",
        &good,
    );
    let broken = entry("zulu", "zulu-one", "", "not-a-digest");
    fs::write(&paths.config, setup.clone() + &mismatched + &broken).unwrap();
    let first = Config::read(&paths.config).unwrap_err();
    // Same two defects, aliases swapped in sort order.
    let mismatched = entry(
        "zulu",
        "alpha-one",
        "private_protocol='connectors-private/2'\n",
        &good,
    );
    let broken = entry("alpha", "zulu-one", "", "not-a-digest");
    fs::write(&paths.config, setup + &mismatched + &broken).unwrap();
    let second = Config::read(&paths.config).unwrap_err();
    assert_eq!(
        (first, second),
        (
            Refusal::Failure(Failure::InvalidConfiguration),
            Refusal::Failure(Failure::InvalidConfiguration)
        ),
        "the refusal of a file with another invalid entry depends on alias order"
    );
}
