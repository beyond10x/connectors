//! `story:readme-back-under-its-size-limit`: the README is a front page, so its size is held.
//! Detail beyond the limit lives in the guides it links.

use std::path::Path;

/// The README's limit in bytes (`wc -c README.md`). It was 25,705 when `story:docs-first-run-path`
/// merged; past this, move detail into the guide the README links instead of raising it.
const README_LIMIT_BYTES: usize = 27_000;

#[test]
fn the_readme_stays_under_its_size_limit() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md");
    let size = std::fs::metadata(&path).expect("README.md").len() as usize;
    assert!(
        size <= README_LIMIT_BYTES,
        "README.md is {size} bytes, over its {README_LIMIT_BYTES}-byte limit: move detail to the guide it links"
    );
}
