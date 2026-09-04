---
format: aep.planning-md/1
id: story:retire-the-flux-connectors-identity
kind: story
status: implemented
title: Retire the flux-connectors identity from the artifacts
refs:
- provider: legacy
  reference: S-019
relations:
- derived_from: epic:post-m1
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-resolve
- confidence: cited
  path: web
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-019-retire-the-flux-connectors-identity.md:42`. **read**

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
      `catalog check` ([S-003](../../../docs/stories/S-003-the-lockfile-gets-a-verifier.md)) is green on the new lock, and
      the catalog invariants pass unchanged.
- [x] `grep -ri flux-connectors` over `crates/`, `catalog/`, `web/` and the manifests returns only the
      decided exceptions plus historical references in documentation (design series, stories,
      predecessor citations) — history stays legible, artifacts do not.

## Context

Move this repository's artifacts onto their own identity — in **one** reviewed change whose diff is
the entire catalogue. The predecessor's name and version number are stamped into every canonical
document, every lockfile row, the site's JSON and the wire's `User-Agent`; each is cheap alone and
each rewrites all 55 documents and the pack digest, so doing them separately means paying that review
four times.

Source frontmatter: pillar Catalog · areas [catalog, catalog-build, connector-resolve, web]. **read**

Source `note:` field, quoted: “M1 pinned the identity deliberately: the generator string `flux-connectors <version>` is an INPUT to the migration differential, so the workspace version is held at 0.26.0 and the name kept. The differential has passed; the pin's reason is spent. One story, one full-catalogue rewrite — generator + version + user agent together, not three”

## Status

`done` in the source. Quoted from `docs/stories/S-019-retire-the-flux-connectors-identity.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-019-retire-the-flux-connectors-identity.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-09-02 · 5 revision(s)
- Legacy id `S-019`, recorded as the reference `legacy:S-019`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
