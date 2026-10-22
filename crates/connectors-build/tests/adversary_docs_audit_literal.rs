//! Adversary case for `story:docs-first-run-path` (wave 20261022, unit U2, round 2).
//!
//! An exact-match exemption for `~/.local/state/connectors` is bypassed by how Docusaurus renders
//! a code span followed by text: the HTML closes the `<code>` element and the MDX bundle closes
//! the string literal between the directory and `/owner.sock`. The coordinator's decision: the
//! audit admits that `~/`-rooted directory and any path under it, since it names no user and no
//! repository, and refuses every other `.local/`. These cases run the shipped `docs-audit`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn audit(name: &str, page: &str) -> std::process::Output {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary-docs-audit")
        .join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("page.html"), page).unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors-build"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .args(["docs-audit", "--directory"])
        .arg(&dir)
        .output()
        .unwrap()
}

#[test]
fn a_state_path_split_by_markup_is_admitted() {
    let mut refused = Vec::new();
    for (name, page) in [
        ("html", "<code>~/.local/state/connectors</code>/owner.sock"),
        (
            "mdx",
            "(0,o.jsx)(n.code,{children:\"~/.local/state/connectors\"}),\"/owner.sock\"",
        ),
        ("json", "\"~/.local/state/connectors\\/owner.sock\""),
    ] {
        let output = audit(name, page);
        if !output.status.success() {
            refused.push((page, String::from_utf8_lossy(&output.stderr).into_owned()));
        }
    }
    assert!(
        refused.is_empty(),
        "the audit refuses ~/.local/state/connectors/owner.sock as rendered: {refused:#?}"
    );
}

#[test]
fn every_other_local_path_is_refused() {
    let mut admitted = Vec::new();
    for (name, page) in [
        ("tmp", "cd .local/tmp/build"),
        ("bare", "a bare .local/ path"),
        ("home", "$HOME/.local/state/connectors"),
        ("bin", "~/.local/bin/connectors"),
        ("other", "~/.local/state/other"),
    ] {
        let output = audit(&format!("refused-{name}"), page);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success() || !stderr.contains(".local/") {
            admitted.push(page);
        }
    }
    assert!(
        admitted.is_empty(),
        "the audit does not refuse with the `.local/` marker: {admitted:#?}"
    );
}
