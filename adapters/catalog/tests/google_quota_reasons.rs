//! Every shipped Google selection names Google's quota reasons, so a `403`
//! that means a usage limit reaches the caller as `rate_limited`, not
//! `forbidden`. Google answers an exceeded quota with a `403` whose
//! `error.errors[].reason` is `rateLimitExceeded`, `userRateLimitExceeded` or,
//! for an exhausted daily project quota, `dailyLimitExceeded` (the Calendar and
//! Gmail error guides list all three). The check walks every
//! `providers/google-*` selection set, so a Google provider added later is
//! covered without being named here, and every Google guide names all three.
use serde_json::{Value, json};
use std::{fs, path::Path};

const REASONS: [&str; 3] = [
    "rateLimitExceeded",
    "userRateLimitExceeded",
    "dailyLimitExceeded",
];

#[test]
fn every_google_selection_names_the_quota_reasons() {
    let providers = Path::new(env!("CARGO_MANIFEST_DIR")).join("providers");
    let mut seen = Vec::new();
    let mut missing = Vec::new();
    for entry in fs::read_dir(&providers).unwrap() {
        let directory = entry.unwrap().path();
        let name = directory.file_name().unwrap().to_str().unwrap().to_owned();
        if !name.starts_with("google-") {
            continue;
        }
        let file: Value =
            serde_json::from_slice(&fs::read(directory.join("operations.json")).unwrap()).unwrap();
        for selection in file["operations"].as_array().unwrap() {
            let id = format!("{name} {}", selection["id"]);
            if selection["rate_limit_reasons"] != json!(REASONS) {
                missing.push(id.clone());
            }
            seen.push(id);
        }
    }
    seen.sort();
    assert!(
        seen.iter().any(|id| id.starts_with("google-drive "))
            && seen.iter().any(|id| id.starts_with("google-slides "))
            && seen.iter().any(|id| id.starts_with("google-calendar "))
            && seen.iter().any(|id| id.starts_with("google-gmail ")),
        "{seen:?}"
    );
    assert!(missing.is_empty(), "without {REASONS:?}: {missing:#?}");
}

#[test]
fn every_google_guide_names_the_quota_reasons() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let mut missing = Vec::new();
    for guide in ["drive", "slides", "calendar", "gmail"] {
        let text = fs::read_to_string(docs.join(format!("catalog-google-{guide}.md"))).unwrap();
        for reason in REASONS {
            if !text.contains(&format!("`{reason}`")) {
                missing.push(format!("{guide}: {reason}"));
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}
