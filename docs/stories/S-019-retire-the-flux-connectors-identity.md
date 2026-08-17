---
id: S-019
title: "Retire the flux-connectors identity from the artifacts"
pillar: Catalog
status: done
priority:
design:
epic: post-m1
areas: [catalog, catalog-build, connector-resolve, web]
note: "M1 pinned the identity deliberately: the generator string `flux-connectors <version>` is an INPUT to the migration differential, so the workspace version is held at 0.26.0 and the name kept. The differential has passed; the pin's reason is spent. One story, one full-catalogue rewrite — generator + version + user agent together, not three"
---

# Retire the flux-connectors identity from the artifacts

## Goal

Move this repository's artifacts onto their own identity — in **one** reviewed change whose diff is
the entire catalogue. The predecessor's name and version number are stamped into every canonical
document, every lockfile row, the site's JSON and the wire's `User-Agent`; each is cheap alone and
each rewrites all 55 documents and the pack digest, so doing them separately means paying that review
four times.

## Why the identity is pinned today

M1 pinned it on purpose, and said so in two places:

- `Cargo.toml` `[workspace.package]`: *"**0.26.0 is carried deliberately, and it is load-bearing.**
  The generator identity `flux-connectors <version>` is embedded in every canonical document and in
  every `connectors.lock` row, so it is an *input* to the migration differential design 02 §7 item 6
  requires… It moves when the differential is retired."*
- `crates/catalog-build/src/seam.rs::generator()` —
  `format!("flux-connectors {}", env!("CARGO_PKG_VERSION"))` — carrying the same note, and one more
  that matters: the generator is part of the hash domain `connectors.lock` records, so *a generator
  change must invalidate generated output*, or a stale artifact survives a codegen fix.

The differential has now passed byte-exact. The reason for the pin is spent; the pin is not yet.

`DEFAULT_USER_AGENT` (`crates/connector-resolve/src/request.rs:22`) rides along and is already
half-migrated, which is the tell: it reads `flux-connectors/0.26.0 (+https://github.com/b10x/connectors)`
— the product token from the predecessor, the repository from here.

## Acceptance

- [x] The generator identity names **this** repository, the workspace version is chosen deliberately
      (a fresh line, or a continuation of 0.26.0 — either is defensible), and the `User-Agent` product
      token matches both. All three move in **one commit** with **one** whole-catalogue regeneration:
      55 documents, the pack digest, `connectors.lock`, `web/public/catalog.json`.
- [x] The reasons are replaced, not deleted: `Cargo.toml`'s load-bearing comment and `seam.rs`'s
      generator doc say what the version means **now** (and that the differential is retired), rather
      than losing the paragraph that explained the old pin.
- [x] `DEFAULT_USER_AGENT`'s three assertions in `request.rs` move with the value, so the constant
      stays derived from the manifest (`CARGO_PKG_VERSION`, `CARGO_PKG_REPOSITORY`) and cannot go
      stale at a release.
- [x] The **whole identity inventory is enumerated and each item decided** — renamed now, or kept with
      a written reason. Known members, measured post-M1:
      the pack magic `flux-connectors-catalog-pack` (`crates/catalog-reader/src/lib.rs:70` — renaming
      it is a pack-format break, so decide it here or state why it stays);
      the `connectors.lock` header naming `flux-connectors check` (`connector-spec/src/lock.rs:74-75`);
      the canonical document's `$schema` URL, still
      `https://github.com/codewandler/flux-connectors/catalog/connector-document.schema.json`
      (`catalog-build/src/document.rs:67`);
      `web/package.json`'s `name` and `description`;
      and the site's `PATH_RESOLVER = 'flux-connectors:path-resolver'` injection key.
      A rename that leaves half the strings behind has not retired anything.
- [x] After the rewrite the tree is a fixed point: `catalog build` writes nothing on a second run,
      `catalog check` ([S-003](S-003-the-lockfile-gets-a-verifier.md)) is green on the new lock, and
      the catalog invariants pass unchanged.
- [x] `grep -ri flux-connectors` over `crates/`, `catalog/`, `web/` and the manifests returns only the
      decided exceptions plus historical references in documentation (design series, stories,
      predecessor citations) — history stays legible, artifacts do not.

## Progress

- 2026-08-13 — done in the coordinated schema wave. The product identity is `connectors`; this is
  the first `b10x/connectors` artifact line, version `0.1.0`. Canonical generator strings,
  the derived request `User-Agent`, all internal crate versions, schema identifiers, pack magic,
  package metadata and the site path-resolver key moved together and were regenerated once.
- Inventory decisions: `connectors.lock` already used the product-neutral `catalog build/check`
  commands and stayed; the pack magic changed to `connectors-catalog-pack` because no released
  B10x reader contract existed to preserve. `SOURCES.toml`, `AGENTS.md`, root/provider
  comments and design/story prose retain predecessor citations as history.
- The explorer's `/flux-connectors/` GitHub Pages base, its matching test, and its visual labels are
  deliberately retained until S-018 repairs and republishes the explorer. Renaming those alone
  would produce a broken deployment path and would exceed this artifact-identity story.
- The second build was a fixed point; `catalog diff`, `catalog check`, the whole-catalog invariants,
  reader checks and request identity assertions pass on the new hashes.

## Notes

- **Sequencing:** ride the schema-evolution wave with
  [S-001](S-001-the-document-carries-the-callers-contract.md),
  [S-002](S-002-effects-are-read-never-derived.md) and
  [S-015](S-015-retire-the-quirks-umbrella.md). Each of those already rewrites every committed
  document; folding the identity in makes it **one** full-catalogue rewrite to review instead of
  four. If the wave is sequenced rather than merged, this goes last, so the identity moves once the
  fields have settled.
- The generator being in the lock's hash domain is a feature to preserve, not an obstacle: after this
  story every artifact's row moves, which is the correct signal that the tool that produced them
  changed.
- Naming is not settled here — architecture open question 1 asks whether the binary and product are
  `connectors` or something shorter, and it says the answer is cheap until M2 and expensive after.
  If that decision is close, take it first and stamp the answer once.
