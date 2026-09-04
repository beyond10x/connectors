//! **Adversary pass 3: the fence `docs/design/19-the-cli-surface.md` says is measured.**
//!
//! `story:cli-first-level-groups` grew `crates/connectors-cli/src/lib.rs` past
//! `CLI_TOTAL_LINE_LIMIT`, and its `## Fence` section asked for the cap to be *raised* with a dated
//! reason comment. The diff under review deletes the constant and the assertion that read it from
//! `crates/catalog-build/tests/main/architecture_fence.rs` instead.
//!
//! `docs/design/19-the-cli-surface.md` is not a comment: it is the page this repository keeps to
//! say why `ess/generated/clap/` sits beside the specification rather than inside the crate, and
//! the first of its "Two reasons, both measured" was that cap. Deleting the cap in the same commit
//! that leaves the page asserting it makes the page wrong about the tree, and no step of the gate
//! compares the two — `every_citation_this_unit_wrote_resolves` checks that a cited *line number*
//! exists and says in its own comment that it does not check what the line means.
//!
//! **Resolved by deleting the claim, not by restoring the cap** — the operator's instruction, on
//! the ground that the cap had been raised at every one of the six times it fired and had never
//! moved a line out of the binary. The case below is therefore kept as the *equivalence* its own
//! refusal named, in both directions, rather than as the one-sided assertion it was written as: a
//! case that only asserts the wrong sentence is still there can be answered by deleting the case,
//! which is the failure this whole pass exists to catch, one level up.

use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// **The page and the architecture fence agree about whether the frontend is capped.**
///
/// The page's own sentence is the specification here, quoted rather than paraphrased so that a
/// reader can see the two halves being compared. Both halves of the *measurement* are checked: a
/// constant that exists but that no `assert!` reads is a number, not a fence, which is the shape
/// this repository has already been caught shipping
/// (`crates/connectors-cli/tests/adversary_fence_probe.rs`).
///
/// Three things are refused. The page claiming a cap nothing measures — the finding. A cap that
/// exists while the page says nothing about it — the same defect from the other side, and the one
/// that would let the cap come back unannounced. And the section losing the reason that survives
/// the cap's deletion, so that removing half a two-part answer cannot quietly remove the answer.
#[test]
fn the_cap_the_design_page_says_is_measured_is_declared_and_asserted() {
    let root = repository_root();
    let page_text = read(&root.join("docs/design/19-the-cli-surface.md"));
    let fence_text = read(&root.join("crates/catalog-build/tests/main/architecture_fence.rs"));

    let claim = "the architecture fence caps the thin frontend at `CLI_TOTAL_LINE_LIMIT` \
                 production lines";
    let claimed = page_text.contains(claim);
    let declared = fence_text.contains("const CLI_TOTAL_LINE_LIMIT");
    let asserted = fence_text
        .lines()
        .skip_while(|line| !line.contains("fn product_cli_is_a_thin_frontend"))
        .take_while(|line| !line.starts_with("#[test]") || line.contains("product_cli"))
        .any(|line| line.contains("CLI_TOTAL_LINE_LIMIT"));

    assert_eq!(
        claimed,
        declared && asserted,
        "docs/design/19-the-cli-surface.md carries\n  \"{claim}\"\n: {claimed}, and \
         crates/catalog-build/tests/main/architecture_fence.rs declares the constant: {declared}, \
         reads it inside `product_cli_is_a_thin_frontend`: {asserted}.\nThe page gave that cap as \
         the first of \"Two reasons, both measured\" for keeping `ess/generated/clap/` outside the \
         crate; with nothing measuring it, the reason is unmeasured and the page says otherwise. \
         Either raise the cap the way `story:cli-first-level-groups` `## Fence` asked for, or \
         delete the claim from the page in the same change that deletes the fence — and if the cap \
         is restored, say so on the page in the same change."
    );

    let section = page_text
        .split("## The generated tree is a parallel artifact, not a replacement")
        .nth(1)
        .expect("docs/design/19-the-cli-surface.md keeps the section that reason lives in")
        .split("\n## ")
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        section.contains("`--out` inside the real crate puts a second `Cargo.toml`"),
        "docs/design/19-the-cli-surface.md no longer says why `ess/generated/clap/` sits beside the \
         specification rather than inside the crate. The line-count cap was one of two reasons and \
         is deleted; this is the other, and it was always the one doing the work:\n{section}"
    );
    assert!(
        fence_text.contains("fn product_cli_is_a_thin_frontend"),
        "the page names `product_cli_is_a_thin_frontend` as what bounds the frontend now, and \
         crates/catalog-build/tests/main/architecture_fence.rs does not declare it"
    );
}
