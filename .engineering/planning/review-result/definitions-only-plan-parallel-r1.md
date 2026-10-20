---
format: aep.planning-md/3
id: review-result:definitions-only-plan-parallel-r1
kind: review-result
status: active
title: Plan critic parallel safety, round 1, on the definitions-only drafts
relations:
- reviews: story:definitions-release-index
- reviews: story:definitions-install
- reviews: story:definitions-pin-covers-catalog
- reviews: story:definition-tests-read-pinned-release
- reviews: story:catalog-tests-read-pinned-release-gitlab-engine
- reviews: story:catalog-tests-read-pinned-release-providers
- reviews: story:definitions-fixture-copy-removed
revision: 1
---
needs-revision
story:catalog-tests-read-pinned-release-gitlab-engine — both this story and story:catalog-tests-read-pinned-release-providers must give `adapters/catalog` a dev-dependency on the test-support crate (it is not in the `[dev-dependencies]` of `adapters/catalog/Cargo.toml`). That puts both on `adapters/catalog/Cargo.toml` and the `connectors-catalog-provider` stanza of `Cargo.lock`, and neither Scope names those files or says they share them (inferred; the support crate's location is left to the implementor). Fix by either an ordering edge that records the manifest as its reason, or having `story:definitions-pin-covers-catalog` add the dev-dependency to every consuming crate manifest so neither batch touches it. — .engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md:43, .engineering/planning/story/catalog-tests-read-pinned-release-providers.md:40, .engineering/planning/story/definitions-pin-covers-catalog.md:40

What I read: the 7 drafts and the epic with `aep plan artifact show` and `graph`, `waves` and `list`. I also ran `git grep` at 9201b1177 over `adapters/catalog/tests`, the `Cargo.toml` manifests and `definition_auth.rs`, and compared the two test-file lists against a search for `providers|generated/bundles`.
- **Surfaces.** 7 of 7 are placed. 4 have mostly cited surfaces: the two catalog test batches, `definition-tests` and `fixture-copy`. 3 have partly inferred surfaces: `release-index`, `install` and `pin-covers-catalog`. None are unplaceable.
- **Disjointness.** The two catalog batches are file-disjoint (45 files and 35 files, empty intersection by `comm`). Both bodies say they do not touch `hand_written/`. The `definition_auth.rs` in the `definition-tests` scope is `crates/connectors-catalog/tests/definition_auth.rs`, a different file from the one in `adapters/catalog/tests`.
- **Producers.** I traced 8 acceptance reads to a producer in the set. 6 are direct `depends_on` edges: install→release-index, pin→install, the three test stories→pin, and fixture-copy→those three. The other 2 are transitive only: pin reading the first tag, and `definition-tests` reading the Jira definition at the first tag. Both run through pin→install→release-index.
- **Parallel set.** Once pin lands, `definition-tests` and the two catalog batches can run together, apart from the finding above. The other stories are ordered by edges.

What I could not establish:
- **`aep plan artifact waves`.** It reports the 7 as unassessed, because no frontmatter scope is written yet. It says nothing about these 7.
- **Test-support crate.** I do not know where it will live, so the manifest collision rests on an inference.
- **Out of my lane, acceptance:** `story:definition-tests-read-pinned-release` Context says `definition_import.rs` keeps comparing against in-tree bundles, while its Acceptance says no Scope file reads `adapters/catalog/providers` or `generated/bundles` (definition-tests lines 49-52 against the Acceptance).
- **Out of my lane, design:** `release-index` does not require connectors-definitions PR 4 before the first tag, though `definition_import.rs` is red on Jira without it.
- **Scope note, not a finding:** when frontmatter scope is written, `definition_auth.rs` must be a full path in both stories. `AGENTS.md` and `.github/workflows/` in `release-index` belong to the other repository and would appear to collide with `pin-covers-catalog` (the chain orders them anyway).

```findings
[{"file": ".engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md", "line": 43, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "both this story and story:catalog-tests-read-pinned-release-providers must add a dev-dependency on the test-support crate to adapters/catalog/Cargo.toml (and the matching Cargo.lock stanza), neither Scope names those files or admits the shared manifest, and no edge orders the two; this rests on an inferred surface because the support crate's location is left to the implementor; remedies: an ordering edge recording the manifest as its reason, or have story:definitions-pin-covers-catalog add the dev-dependency to every consuming crate manifest"}]
```
