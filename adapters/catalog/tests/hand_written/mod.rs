//! The native catalog configurations the guides showed before connectors were
//! declared, kept as the configurations a pinned or not yet migrated
//! installation holds. A guide now configures a connector and a named
//! instance; these files keep the fixtures that ran its configuration, and the
//! revisions they digest to, as they were. `<page>.<n>.json` is the `n`th
//! configuration `docs/<page>.md` showed; `<page>.write-values.json` holds the
//! values a write instance changed in the page's read configuration.
#![allow(dead_code)]

use std::{fs, path::Path};

fn directory() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/hand_written")
}

/// The hand-written configurations of the guide `page` (`catalog-jira.md`),
/// as JSON text, in the order the guide showed them.
pub fn blocks(page: &str) -> Vec<String> {
    let stem = page.strip_suffix(".md").unwrap_or(page);
    let mut numbered: Vec<(usize, String)> = fs::read_dir(directory())
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().to_str().unwrap().to_owned();
            let number = name
                .strip_prefix(&format!("{stem}."))?
                .strip_suffix(".json")?
                .parse()
                .ok()?;
            Some((number, name))
        })
        .collect();
    numbered.sort();
    numbered
        .into_iter()
        .map(|(_, name)| fs::read_to_string(directory().join(name)).unwrap())
        .collect()
}

/// The JSON blocks a guide documents: those still in `docs/<page>`, then its
/// hand-written configurations.
pub fn documented_blocks(page: &str) -> Vec<String> {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs")
            .join(page),
    )
    .unwrap();
    let mut found: Vec<String> = text
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body.to_owned()))
        .collect();
    found.extend(blocks(page));
    found
}

/// One file of the directory by name, such as
/// `catalog-google-gmail.write-values.json`.
pub fn file(name: &str) -> String {
    fs::read_to_string(directory().join(name)).unwrap()
}
