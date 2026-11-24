//! The whole-value pattern rule on the vectors it shares with the host
//! (story:standing-grant-binds-a-pattern-or-a-value-set, acceptance 26).
//!
//! `connectors-host` has a copy of [`pattern_matches`] for the target members
//! of a standing grant (`local::standing`). Both crates read
//! `crates/connectors-host/tests/fixtures/whole_value_patterns.json`, so the
//! two copies cannot differ on those vectors unseen.
use connectors_catalog::inventory::pattern_matches;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vectors {
    #[serde(rename = "about")]
    _about: String,
    vectors: Vec<Vector>,
}

/// What the rule answers for `value` under `pattern`; `matches` is `null`
/// where the rule refuses the pattern itself.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vector {
    pattern: String,
    value: String,
    matches: Option<bool>,
}

#[test]
fn the_shared_pattern_vectors_hold() {
    let shared: Vectors = serde_json::from_str(include_str!(
        "../../connectors-host/tests/fixtures/whole_value_patterns.json"
    ))
    .unwrap();
    assert!(shared.vectors.len() >= 30, "{}", shared.vectors.len());
    for vector in shared.vectors {
        assert_eq!(
            pattern_matches(&vector.pattern, &vector.value),
            vector.matches,
            "{:?} on {:?}",
            vector.pattern,
            vector.value
        );
    }
}
