---
format: aep.planning-md/3
id: story:definitions-install
kind: story
status: implemented
title: connectors definitions install fetches a connectors-definitions tag, verifies it against index.json, and records a lock and a cache
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-release-index
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/Cargo.toml
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src/local
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: cited
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: inferred
  path: ess/system.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T20:20:57Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-29T20:20:57Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-30T05:39:52Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
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

- The published index, from connectors-definitions 0.3.0 on (`spec/domains/release.yaml` there): `schema` (`urn:beyond10x:connectors-definitions-index:1`, the format version D5 refuses when unknown), `tag`, `commit`, `features` (sorted; at 0.3.0 `embedded_text`, `query_fixed`, `query_folds`, `rate_limit_headers`, the names an unmet feature refuses), `asset` {`name`, `sha256`} and `files` [{`path`, `sha256`, `size`}], one entry per regular file of the asset, sorted by byte order, paths relative without the tarball prefix. Release assets are never replaced. 0.1.0 and 0.2.0 have no index.

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

Derived 2026-10-29 by `story-scoper` on origin/main 65c928d72. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** a new `connectors definitions install` command in `apps/connectors`, built on a fetch-and-verify library crate — cited (story Context: "lives in a library crate, not only in `apps/connectors`")
- **Files:** `apps/connectors/spec/cli.yaml` — cited (story Context)
- **Files:** `ess/domains/cli.yaml` — cited (story Context; owned by `contracts/cli/v1alpha1/semantics.md`, `ess/domains/cli.yaml:4`)
- **Files:** `contracts/cli/v1alpha1/semantics.md`, `contracts/cli/v1alpha1/scenarios.md` — cited (story Scope)
- **Files:** `apps/connectors-cli-contract/` (generated from `cli.yaml` by `connectors-build`, `crates/connectors-build/src/cli.rs:13-20`) — cited
- **Files:** `website/docs/reference/cli.md` (generated page, `crates/connectors-docs/src/cli.rs:10`) — cited
- **Also likely:** `apps/connectors/src/local.rs` — inferred: the new callable is routed in `LocalHandler::dispatch` (`:158`) and probably admitted without a configuration in `configuration_preflight` (`:485`)
- **Also likely:** a new module in `apps/connectors/src/local/` and `apps/connectors/Cargo.toml` — inferred
- **Also likely:** a new crate under `crates/` holding the fetch-and-verify core, with its `[workspace.dependencies]` entry in `Cargo.toml` and `Cargo.lock` — inferred. `members` already globs `crates/*` (`Cargo.toml:4`). No `tar` workspace dependency exists today.
- **Also likely:** `crates/connectors-host/src/local/config.rs` — inferred: design section 6 puts the cache under `<data dir>`, and `Paths` has only `config` and `state` (`:1109-1112`)
- **Also likely:** a new ESS domain file next to `ess/domains/cli.yaml` for lock, index and cache entry, plus its entry in `ess/system.yaml` (`:5-30`) — inferred (the story says the placement is inferred)
- **Also likely:** `apps/connectors/tests/` (a new install test with a release served from a local directory) — inferred
- **Also likely:** `CHANGELOG.md` (`## Unreleased`) — inferred
- **Documents:** the two CLI contract documents above. The acceptance requires no guide. — cited
- **Confidence:** medium — the CLI binding and contract files are cited, but the crate split, the host paths and the ESS placement are left to implementation
- **Would collide with:** any unit that changes the `connectors` command tree (`apps/connectors/spec/cli.yaml`, which regenerates `apps/connectors-cli-contract/` and `website/docs/reference/cli.md`), the callable routing in `apps/connectors/src/local.rs`, `ess/domains/cli.yaml`, the `contracts/cli/v1alpha1/` semantics and scenarios, or the workspace `Cargo.toml`/`Cargo.lock`. In flight on 2026-10-29: the D218 invoke units change `apps/connectors/spec/cli.yaml` and its generated contract and page; the D215 branch changes `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`.
- **Safety fact:** the install cache is a different namespace from `<state>/definitions/`, the only definitions cache `Config::read_at` merges into a configuration (`crates/connectors-host/src/local/definitions.rs:4-10,36`). Until `story:instances-resolve-through-locked-index` lands, an install cannot change what any configured instance resolves to. Proven to level 2 (file:line); unproven at run time.
- **Not established:** the new crate's path; the new ESS domain file's name; the cache location (`<data dir>` against `Paths`); whether release asset downloads redirect (the host HTTP client follows no redirect, `crates/connectors-host/src/http.rs:178-179`).

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
- Not built with it (SM-D209): installing a release whose tag and connectors this binary was not
  built with succeeds without a rebuild, provided its format version and features are supported.
  Install reads nothing embedded at build time except the format versions and features this build
  supports.

## Decisions

- SM-D209 (operator, 2026-10-29): "make sure that the stuff from connectors-definitions can be
  read on-the-fly". A running binary reads an installed connectors-definitions release; adding or
  changing a catalog connector needs a definitions release and an install, not a connectors
  rebuild.
- SM-D210 (coordinating session, 2026-10-29): SM-D209 does not retire option C of
  `decision-blocker:catalog-first-run-source`. The pinned release embedded at build time stays the
  first-run default before any install; an installed tag resolves at run time on top of it, and
  `story:instances-resolve-through-locked-index` carries that resolution. This story's install
  therefore writes the lock and cache a running binary reads, and the new acceptance case holds it.
- D247 (orchestrator, 2026-10-30): "B, as you proposed. Keep the four in FEATURES, state the
  limitation in the CHANGELOG and the doc comment, and go on with your release. C0 will carry the
  test that FEATURES equals what the lowerer accepts." The proposal (this wave's coordinating
  session): install admits `embedded_text`, `query_fixed`, `query_folds` and `rate_limit_headers`
  by name, while the definition lowerer (`crates/connectors-catalog/src/definition.rs:485`, `:712`)
  still refuses all four; no instance reads the lock yet, so nothing breaks today. Lowerer support
  for the four goes to the next connectors wave (D243 C0).
