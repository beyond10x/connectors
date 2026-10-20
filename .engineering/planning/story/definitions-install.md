---
format: aep.planning-md/3
id: story:definitions-install
kind: story
status: draft
title: connectors definitions install fetches a connectors-definitions tag, verifies it against index.json, and records a lock and a cache
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-release-index
revision: 3
---
## Outcome

`connectors definitions install` fetches a named connectors-definitions tag from a configured
source, consuming exactly what `story:definitions-release-index` publishes: the tag, the release
asset with the files of the tagged tree, and `index.json`. It checks every file against
`index.json`, records source, tag, commit and index SHA-256 in a lock file, and stores the files in
a local cache keyed by the index SHA-256, read-only once verified. A digest mismatch, an unknown
format version and an unmet feature are each a refusal by name. Epic step 1.

## Context

- ADR D5: "A command fetches a named tag from a configured source, checks every file against
  `index.json`, records source, tag, commit and index digest in a lock file, and stores the files
  in a local cache keyed by the index digest. [...] Install and update grant nothing: no
  connection, no operation admission, no OAuth exchange, no write authority. A digest mismatch,
  an unknown format version or an unmet feature is a refusal by name. Nothing updates
  implicitly."
- The refusals are the three D5 names. Design section 6 (draft) adds "a tag that moved to another
  commit"; it is not in D5 or the epic and not in this story.
- Design section 6 (draft) proposes `connectors definitions install --source <git url> --ref
  <tag>`, `definitions.lock` in the connectors config directory and the cache
  `<data dir>/connectors/definitions/<index sha256>/`.
- No install or fetch step exists (ADR Context; no `definitions install` in `apps/`, `crates/` or
  `contracts/` on 9201b1177).
- Commands are declared in `apps/connectors/spec/cli.yaml` (`ess-cli/1`) and their payloads in
  `ess/domains/cli.yaml`, owned by `contracts/cli/v1alpha1/semantics.md` (`ess/domains/cli.yaml:4`).
- The host already keeps invoked definition files under `<state>/definitions/<digest>/` with
  `instances.toml` beside them (D12 point 3; `crates/connectors-host/src/local/definitions.rs:4-7,36`).
  The install cache does not share that namespace (inferred).
- The host HTTP client follows no redirect and uses no proxy
  (`crates/connectors-host/src/http.rs:178-179`); release asset downloads may answer with a
  redirect (not verified). How install fetches the asset is this story's implementation and is
  recorded at delivery. Tests serve a release from a local directory and need no network (design
  section 11, M3: "install from a local directory and from a git tag").
- The fetch-and-verify core has two callers besides this command, so it lives in a library crate,
  not only in `apps/connectors` (inferred): tests in other crates, and the fetch of the pinned
  release that runs before cargo (both added by `story:definitions-pin-covers-catalog`). Under
  option C of `decision-blocker:catalog-first-run-source`, `build.rs` embeds what that fetch
  wrote (`story:instances-resolve-through-locked-index`), so the fetch runs before
  `connectors-catalog-provider` can build: the crate holding the core does not depend on it,
  directly or through `connectors-build` (`crates/connectors-build/Cargo.toml:11`), and can be
  run on its own (inferred).
- The reference lowerer is `crates/connectors-catalog/src/definition.rs` (`lower()`, ADR D3).
  When installed definitions are lowered is not decided by this story.

## Domain relations

- Lock to release: one lock per connectors configuration pins one source, tag, commit and index
  SHA-256 ("the locked index", ADR D5); the release outlives the lock. Decided by ADR D5 and
  design section 6 (draft). No `ess/1` document declares it; this story declares lock, index and
  cache entry in ESS beside `ess/domains/cli.yaml` (placement inferred).
- Cache entry to release: one entry per installed index SHA-256; several may coexist; read-only
  once verified (ADR D5, design section 6). Removing old entries is not decided and not in this
  story.
- Install to connection, operation admission, OAuth exchange, write authority: none (ADR D5,
  D12 point 4).

## Scope

- `apps/connectors/spec/cli.yaml` (cited)
- `ess/domains/cli.yaml` (cited)
- `contracts/cli/v1alpha1/semantics.md`, `contracts/cli/v1alpha1/scenarios.md` (cited)
- `apps/connectors/src/local/` (inferred: a command module beside `definition.rs`)
- `crates/connectors-host/src/local/` (inferred: lock and cache paths beside `definitions.rs`)
- `apps/connectors/tests/` (inferred)
- `website/docs/reference/` (inferred: the generated CLI reference)

## Not in this story

- Instances resolving a connector through the locked index (last sentence of D5):
  `story:instances-resolve-through-locked-index`.
- What a binary serves before any install: the pinned release it embeds (option C),
  `story:instances-resolve-through-locked-index`.
- `list`, `verify`, `update` (design section 6), re-consent on a profile change, D6.

## Acceptance

Each against a release served from a local directory in tests:

- Success: `connectors definitions install` writes a lock naming source, tag, commit and index
  SHA-256, and a read-only cache keyed by that SHA-256 holding every file the index names.
- Digest mismatch: a file whose SHA-256 differs from its `index.json` entry ends in a refusal by
  name, and the lock and the cache are as they were before the command.
- Unknown format version: an `index.json` of a format version this build does not read ends in a
  refusal by name, and the lock and the cache are as they were before the command.
- Unmet feature: a release requiring a feature this build does not support ends in a refusal by
  name, and the lock and the cache are as they were before the command.
